use crate::{
    types::{Address, Block, BlockHash, OutPoint, TxOut},
    UtxoSet,
};
use ic_btc_types::Height;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A cache maintaining data related to outpoints in unstable blocks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutPointsCache {
    /// Caches outpoints and their corresponding transaction outputs.
    tx_outs: BTreeMap<OutPoint, TxOutInfo>,

    /// Caches the outpoints added for each address in a block.
    added_outpoints: BTreeMap<BlockHash, BTreeMap<Address, Vec<OutPoint>>>,

    /// Caches the outpoints removed for each address in a block.
    removed_outpoints: BTreeMap<BlockHash, BTreeMap<Address, Vec<OutPoint>>>,
}

impl OutPointsCache {
    pub fn new() -> Self {
        Self {
            tx_outs: BTreeMap::new(),
            added_outpoints: BTreeMap::new(),
            removed_outpoints: BTreeMap::new(),
        }
    }

    /// Retrieves the list of outpoints that were added for the given address in the given block.
    pub fn get_added_outpoints(&self, block_hash: &BlockHash, address: &Address) -> &[OutPoint] {
        self.added_outpoints
            .get(block_hash)
            .map(|address_utxos| {
                address_utxos
                    .get(address)
                    .map(|outpoints| outpoints.as_slice())
                    .unwrap_or(&[])
            })
            .unwrap_or(&[])
    }

    /// Retrieves the list of outpoints that were removed for the given address in the given block.
    pub fn get_removed_outpoints(&self, block_hash: &BlockHash, address: &Address) -> &[OutPoint] {
        self.removed_outpoints
            .get(block_hash)
            .map(|address_utxos| {
                address_utxos
                    .get(address)
                    .map(|outpoints| outpoints.as_slice())
                    .unwrap_or(&[])
            })
            .unwrap_or(&[])
    }

    /// Retrieves the `TxOut` associated with the given `outpoint`, along with its height.
    pub fn get_tx_out(&self, outpoint: &OutPoint) -> Option<(&TxOut, Height)> {
        self.tx_outs
            .get(outpoint)
            .map(|info| (&info.txout, info.height))
    }

    /// Inserts the outpoints in a block, along with their transaction outputs, into the cache.
    pub fn insert(
        &mut self,
        utxos: &UtxoSet,
        block: &Block,
        height: Height,
    ) -> Result<(), TxOutNotFound> {
        // A map to store all the transaction outputs referenced by the given block.
        let mut tx_outs: BTreeMap<OutPoint, TxOutInfo> = BTreeMap::new();
        let mut removed_outpoints = BTreeMap::new();
        let mut added_outpoints = BTreeMap::new();

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
                            .get_utxo(&outpoint)
                            .ok_or_else(|| TxOutNotFound(outpoint.clone()))?,
                    },
                };

                if let Ok(address) = Address::from_script(
                    &bitcoin::Script::from(txout.script_pubkey.clone()),
                    utxos.network(),
                ) {
                    let entry = removed_outpoints.entry(address).or_insert(vec![]);
                    entry.push(outpoint.clone());
                }

                let entry = tx_outs.entry(outpoint).or_insert(TxOutInfo {
                    txout,
                    height,
                    count: 0,
                });
                entry.count += 1;
            }

            // Outputs can be inserted as-is into the cache, maintaining a count of how
            // many we inserted into the cache that reference them.
            for (i, txout) in tx.output().iter().enumerate() {
                let outpoint = OutPoint {
                    txid: tx.txid(),
                    vout: i as u32,
                };

                if let Ok(address) = Address::from_script(&txout.script_pubkey, utxos.network()) {
                    let entry = added_outpoints.entry(address).or_insert(vec![]);
                    entry.push(outpoint.clone());
                }

                // Retrieve the associated entry in the cache and increment its count.
                let entry = tx_outs.entry(outpoint.clone()).or_insert(TxOutInfo {
                    txout: txout.into(),
                    height,
                    count: 0,
                });
                entry.count += 1;
            }
        }

        // Merge all the transaction outputs of this block into the cache.
        for (outpoint, tx_out_info) in tx_outs {
            self.tx_outs
                .entry(outpoint)
                .and_modify(|t| t.count += tx_out_info.count)
                .or_insert(tx_out_info);
        }

        self.added_outpoints
            .insert(block.block_hash(), added_outpoints);
        self.removed_outpoints
            .insert(block.block_hash(), removed_outpoints);

        Ok(())
    }

    /// Removes the outpoints of a block from the cache.
    ///
    /// Note that an outpoint can be referenced by multiple blocks, so an outpoint is only removed
    /// from the cache when there are no more blocks referencing it.
    pub fn remove(&mut self, block: &Block) {
        fn decrement_count_and_maybe_remove(cache: &mut OutPointsCache, outpoint: &OutPoint) {
            let entry = cache.tx_outs.get_mut(outpoint).unwrap_or_else(|| {
                panic!(
                    "outpoint {:?} must be present in the outpoints cache.",
                    outpoint
                )
            });

            // Decrement the value's count.
            entry.count -= 1;

            // Remove the outpoint if there are no more blocks in the cache referencing it.
            if entry.count == 0 {
                cache.tx_outs.remove(outpoint);
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

        let block_hash = block.block_hash();
        self.added_outpoints.remove(&block_hash);
        self.removed_outpoints.remove(&block_hash);
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
        let cache = OutPointsCache::new();
        assert_eq!(cache.tx_outs, maplit::btreemap! {},);
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
        let mut cache = OutPointsCache::new();

        // Insert the genesis block and verify
        cache.insert(&utxos, &block_0, 0).unwrap();

        // The cache contains the outpoint of block 0.
        let outpoint_0 = OutPoint {
            txid: tx_0.txid(),
            vout: 0,
        };
        assert_eq!(
            cache.tx_outs,
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

        cache.insert(&utxos, &block_1, 1).unwrap();

        let outpoint_1 = OutPoint {
            txid: tx_1.txid(),
            vout: 0,
        };

        // The outpoints info cache contains the outpoints of block 0 and block 1.
        assert_eq!(
            cache,
            OutPointsCache {
                tx_outs: maplit::btreemap! {
                    outpoint_0.clone() => TxOutInfo {
                        txout: (&tx_0.output()[0]).into(),
                        height: 0,
                        count: 2
                    },
                    outpoint_1.clone() => TxOutInfo {
                        txout: (&tx_1.output()[0]).into(),
                        height: 1,
                        count: 1
                    }
                },
                added_outpoints: maplit::btreemap! {
                    block_0.block_hash() => maplit::btreemap! {
                        address_1.clone() => vec![OutPoint::new(tx_0.txid(), 0)]
                    },
                    block_1.block_hash() => maplit::btreemap! {
                        address_2.clone() => vec![OutPoint::new(tx_1.txid(), 0)]
                    },
                },
                removed_outpoints: maplit::btreemap! {
                    block_0.block_hash() => maplit::btreemap! {},
                    block_1.block_hash() => maplit::btreemap! {
                        address_1.clone() => vec![OutPoint::new(tx_0.txid(), 0)]
                    },
                },
            }
        );

        cache.remove(&block_0);

        assert_eq!(
            cache,
            OutPointsCache {
                tx_outs: maplit::btreemap! {
                    outpoint_0 => TxOutInfo {
                        txout: (&tx_0.output()[0]).into(),
                        height: 0,
                        count: 1
                    },
                    outpoint_1 => TxOutInfo {
                        txout: (&tx_1.output()[0]).into(),
                        height: 1,
                        count: 1
                    }
                },
                added_outpoints: maplit::btreemap! {
                    block_1.block_hash() => maplit::btreemap! {
                        address_2 => vec![OutPoint::new(tx_1.txid(), 0)]
                    },
                },
                removed_outpoints: maplit::btreemap! {
                    block_1.block_hash() => maplit::btreemap! {
                        address_1 => vec![OutPoint::new(tx_0.txid(), 0)]
                    },
                },
            }
        );

        // Removing block 1 makes the cache empty again.
        cache.remove(&block_1);
        assert_eq!(
            cache,
            OutPointsCache {
                tx_outs: maplit::btreemap! {},
                added_outpoints: maplit::btreemap! {},
                removed_outpoints: maplit::btreemap! {}
            }
        );
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
        let mut cache = OutPointsCache::new();

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
            cache.insert(&utxos, &block_1, 1),
            Err(TxOutNotFound(outpoint_0))
        );
    }

    #[test]
    fn inserting_a_block_is_atomic() {
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
        let mut cache = OutPointsCache::new();

        cache.insert(&utxos, &block_0, 0).unwrap();

        // The outpoint of block 0.
        let outpoint_0 = OutPoint {
            txid: tx_0.txid(),
            vout: 0,
        };

        // An outpoint that doesn't exist. A block containing this should fail.
        let faulty_outpoint = OutPoint {
            txid: tx_0.txid(),
            vout: 1,
        };

        // Insert a block that consumes the output of the genesis block.
        let tx_1 = TransactionBuilder::new()
            .with_input(outpoint_0.clone())
            .with_input(faulty_outpoint.clone())
            .with_output(&address_2, 2000)
            .build();

        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx_1)
            .build();

        // Inserting the block fails, as its referencing a faulty outpoint.
        assert_eq!(
            cache.insert(&utxos, &block_1, 1),
            Err(TxOutNotFound(faulty_outpoint))
        );

        // The cache doesn't contain anything from block 1
        assert_eq!(
            cache,
            OutPointsCache {
                tx_outs: maplit::btreemap! {
                    outpoint_0 => TxOutInfo {
                        txout: (&tx_0.output()[0]).into(),
                        height: 0,
                        count: 1
                    },
                },
                added_outpoints: maplit::btreemap! {
                    block_0.block_hash() => maplit::btreemap! {
                        address_1 => vec![OutPoint::new(tx_0.txid(), 0)]
                    },
                },
                removed_outpoints: maplit::btreemap! {
                    block_0.block_hash() => maplit::btreemap! {}
                },
            }
        );
    }
}
