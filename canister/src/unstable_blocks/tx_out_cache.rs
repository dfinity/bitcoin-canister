use crate::{
    state::UtxoSet,
    types::{Block, OutPoint, TxOut},
};
use ic_btc_types::Height;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Caches outpoints and their corresponding transaction outputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TxOutCache(BTreeMap<OutPoint, TxOutInfo>);

impl TxOutCache {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Retrieves the `TxOut` associated with the given `outpoint`, along with its height.
    pub fn get_tx_out(&self, outpoint: &OutPoint) -> Option<(&TxOut, Height)> {
        self.0.get(outpoint).map(|info| (&info.txout, info.height))
    }

    /// Inserts the outpoints in a block, along with their transaction outputs, into the cache.
    pub fn insert(&mut self, utxos: &UtxoSet, block: &Block) -> Result<(), TxOutNotFound> {
        // A map to store all the transaction outputs referenced by the given block.
        let mut tx_outs: BTreeMap<OutPoint, TxOutInfo> = BTreeMap::new();

        // The inputs of a transaction contain outpoints that reference the previous
        // outputs that it is consuming. These outputs can be retrieved from a number
        // of sources:
        //
        // 1. From the UTXO set, if the outpoint references a tx in a stable block.
        //
        // 2. From the block, if the outpoint references a previous tx in the same block.
        //
        // 3. From the cache itself, if the outpoint references a tx in an unstable block.
        //    The assumption here is that this cache already contains all the outpoints
        //    referenced by the unstable blocks.
        for tx in block.txdata() {
            for input in tx.input() {
                if input.previous_output.is_null() {
                    continue;
                }

                let outpoint = (&input.previous_output).into();

                // Lookup the `TxOut` in the current cache.
                let (txout, height) = match self.get_tx_out(&outpoint) {
                    Some((txout, height)) => (txout.clone(), height),

                    // Lookup the `TxOut` in the current block.
                    None => match tx_outs.get(&outpoint) {
                        Some(e) => (e.txout.clone(), e.height),

                        // Lookup the `TxOut` in the UTXO set.
                        None => utxos
                            .utxos
                            .get(&outpoint)
                            .ok_or_else(|| TxOutNotFound(outpoint.clone()))?,
                    },
                };

                let entry = tx_outs.entry(outpoint).or_insert(TxOutInfo {
                    txout,
                    height,
                    count: 0,
                });
                entry.count += 1;
            }

            // Outputs can be inserted as-is into the cache, maintaining a count of how
            // we inserted into the cache that reference them.
            for (i, txout) in tx.output().iter().enumerate() {
                let outpoint = OutPoint {
                    txid: tx.txid(),
                    vout: i as u32,
                };

                // Retrive the associated entry in the cache and increment its count.
                let entry = tx_outs.entry(outpoint.clone()).or_insert(TxOutInfo {
                    txout: txout.into(),
                    height: utxos.next_height,
                    count: 0,
                });
                entry.count += 1;
            }
        }

        // Merge all the transaction outputs of this block into the cache.
        for (outpoint, tx_out_info) in tx_outs {
            self.0
                .entry(outpoint)
                .and_modify(|t| t.count += tx_out_info.count)
                .or_insert(tx_out_info);
        }

        Ok(())
    }

    /// Removes the outpoints of a block from the cache.
    ///
    /// Note that an outpoint can be referenced by multiple blocks, so an outpoint is only removed
    /// from the cache when there are no more blocks referencing it.
    pub fn remove(&mut self, block: &Block) {
        fn decrement_count_and_maybe_remove(cache: &mut TxOutCache, outpoint: &OutPoint) {
            let entry = cache.0.get_mut(outpoint).unwrap_or_else(|| {
                panic!(
                    "outpoint {:?} must be present in the outpoints cache.",
                    outpoint
                )
            });

            // Decrement the value's count.
            entry.count -= 1;

            // Remove the outpoint if there are no more blocks in the cache referencing it.
            if entry.count == 0 {
                cache.0.remove(outpoint);
            }
        }

        for tx in block.txdata() {
            for input in tx.input() {
                if input.previous_output.is_null() {
                    continue;
                }

                let outpoint = (&input.previous_output).into();
                decrement_count_and_maybe_remove(self, &outpoint);
            }

            for (i, _) in tx.output().iter().enumerate() {
                decrement_count_and_maybe_remove(
                    self,
                    &OutPoint {
                        txid: tx.txid(),
                        vout: i as u32,
                    },
                );
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct TxOutNotFound(OutPoint);

// A wrapper that stores a `TxOut` along with metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TxOutInfo {
    txout: TxOut,

    // The height of the block that contains the `TxOut`.
    height: Height,

    // The number of blocks that are referencing this `TxOut`.
    //
    // Normally this number would be <= 2, where a block would contain this `TxOut` in its outputs
    // and another block would contain it in its inputs. However, in the case of forks, this
    // count can be larger.
    count: u32,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder},
        types::Network,
    };

    #[test]
    fn empty_when_initialized() {
        let cache = TxOutCache::new();
        assert_eq!(cache.0, maplit::btreemap! {},);
    }

    #[test]
    fn caches_outpoint_info_of_blocks() {
        let network = Network::Mainnet;
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        let tx_0 = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::genesis()
            .with_transaction(tx_0.clone())
            .build();

        let utxos = UtxoSet::new(network);
        let mut cache = TxOutCache::new();

        // Insert the genesis block and verify
        cache.insert(&utxos, &block_0).unwrap();

        // The cache contains the outpoint of block 0.
        let outpoint_0 = OutPoint {
            txid: tx_0.txid(),
            vout: 0,
        };
        assert_eq!(
            cache.0,
            maplit::btreemap! {
                outpoint_0.clone() => TxOutInfo {
                    txout: (&tx_0.output()[0]).into(),
                    height: 0,
                    count: 1
                }
            }
        );

        // Insert a block that consumes the output of the genesis block.
        let tx_1 = TransactionBuilder::new()
            .with_input(outpoint_0.clone())
            .with_output(&address_2, 2000)
            .build();

        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx_1.clone())
            .build();

        cache.insert(&utxos, &block_1).unwrap();

        let outpoint_1 = OutPoint {
            txid: tx_1.txid(),
            vout: 0,
        };

        // The outpoints info cache contains the outpoints of block 0 and block 1.
        assert_eq!(
            cache.0,
            maplit::btreemap! {
                outpoint_0.clone() => TxOutInfo {
                    txout: (&tx_0.output()[0]).into(),
                    height: 0,
                    count: 2
                },
                outpoint_1.clone() => TxOutInfo {
                    txout: (&tx_1.output()[0]).into(),
                    height: 0,
                    count: 1
                }
            }
        );

        cache.remove(&block_0);

        assert_eq!(
            cache.0,
            maplit::btreemap! {
                outpoint_0 => TxOutInfo {
                    txout: (&tx_0.output()[0]).into(),
                    height: 0,
                    count: 1
                },
                outpoint_1 => TxOutInfo {
                    txout: (&tx_1.output()[0]).into(),
                    height: 0,
                    count: 1
                }
            }
        );

        // Removing block 1 makes the cache empty again.
        cache.remove(&block_1);
        assert_eq!(cache.0, maplit::btreemap! {},);
    }

    #[test]
    fn errors_if_tx_out_is_not_found() {
        let network = Network::Mainnet;
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        let tx_0 = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::genesis()
            .with_transaction(tx_0.clone())
            .build();

        let utxos = UtxoSet::new(network);
        let mut cache = TxOutCache::new();

        // The outpoint of block 0.
        let outpoint_0 = OutPoint {
            txid: tx_0.txid(),
            vout: 0,
        };

        // Insert a block that consumes the output of the genesis block.
        let tx_1 = TransactionBuilder::new()
            .with_input(outpoint_0.clone())
            .with_output(&address_2, 2000)
            .build();

        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx_1)
            .build();

        assert_eq!(
            cache.insert(&utxos, &block_1),
            Err(TxOutNotFound(outpoint_0))
        );
    }
}
