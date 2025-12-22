use crate::{
    api::get_utxos,
    heartbeat,
    runtime::{set_successors_responses, GetSuccessorsReply},
    test_utils::BlockChainBuilder,
    types::{GetSuccessorsCompleteResponse, GetSuccessorsResponse, GetUtxosRequest},
    CanisterArg,
};
use async_std::task::block_on;
use ic_btc_interface::{Network, UtxosFilter};
use ic_btc_types::Block;
use proptest::prelude::*;

const ADDRESS: &str = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8";

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn single_chain(
        chain_len in 1..10u32,
    ) {
        crate::init(CanisterArg::Init(crate::InitConfig {
            stability_threshold: Some(10),
            network: Some(Network::Regtest),
            ..Default::default()
        }));

        // Creates a single chain.
        let chain = BlockChainBuilder::new(chain_len).build();

        ingest_blocks(chain.iter());

        // Assert that the tip height/block is equivalent to the depth of the block, which is the
        // standard way of counting confirmations.
        let res = get_utxos(GetUtxosRequest {
            address: ADDRESS.to_string(),
            filter: None,
        }).unwrap();

        assert_eq!(res.tip_height, chain_len - 1);
        assert_eq!(&res.tip_block_hash, chain[chain_len as usize - 1].block_hash().as_bytes());

        for i in 1..chain_len {
            let res = get_utxos(GetUtxosRequest {
                address: ADDRESS.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(i)),
            }).unwrap();

            let block_depth = chain_len - i;
            assert_eq!(res.tip_height, block_depth);
            assert_eq!(&res.tip_block_hash, chain[block_depth as usize].block_hash().as_bytes());
        }
    }
}

proptest! {
    // Tests how the presence of a fork impacts the confirmation count of the main chain.
    //
    // An arbitary main chain is created and a fork at a random location of the chain is created
    // such that the fork is always shorted the main chain's tip.
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn single_fork(
        chain_len in 9..10u32,
        fork_idx in 0..7usize,
    ) {
        let fork_len: u32 = 1;

        // The height of the block present in the fork.
        let fork_height: u32 = fork_idx as u32 + fork_len;

        crate::init(CanisterArg::Init(crate::InitConfig {
            stability_threshold: Some(10),
            network: Some(Network::Regtest),
            ..Default::default()
        }));

        // Creates a chain with the fork.
        let chain = BlockChainBuilder::new(chain_len).build();
        let fork = BlockChainBuilder::fork(&chain[fork_idx], fork_len).build();

        ingest_blocks(chain.iter().chain(fork.iter()));

        // The tip of the chain should be that of the main chain.
        let res = get_utxos(GetUtxosRequest {
            address: ADDRESS.to_string(),
            filter: None,
        }).unwrap();

        assert_eq!(res.tip_height, chain_len - 1);
        assert_eq!(&res.tip_block_hash, chain[chain_len as usize - 1].block_hash().as_bytes());

        for i in 1..chain_len {
            let res = get_utxos(GetUtxosRequest {
                address: ADDRESS.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(i)),
            }).unwrap();

            let block_depth = chain_len - i;

            // For all the confirmations, we expect them to be the depth of the chain with the
            // exception of the the height where the fork block is present. In that case, the
            // tip height is expected to be the depth of the block - 1.
            let expected_height = if block_depth == fork_height {
                block_depth - 1
            } else {
                block_depth
            };

            assert_eq!(res.tip_height, expected_height);
            assert_eq!(&res.tip_block_hash, chain[expected_height as usize].block_hash().as_bytes());
        }
    }
}

#[async_std::test]
async fn multiple_forks() {
    crate::init(CanisterArg::Init(crate::InitConfig {
        stability_threshold: Some(10),
        network: Some(Network::Regtest),
        ..Default::default()
    }));

    // Create a main chain that has two forks.
    let a = BlockChainBuilder::new(7).build();
    let b = BlockChainBuilder::fork(&a[1], 3).build();
    let c = BlockChainBuilder::fork(&a[3], 2).build();

    ingest_blocks(a.iter().chain(b.iter()).chain(c.iter()));

    // The tip of the main chain should be the tip of the "a" chain (i.e. a[6])
    let res = get_utxos(GetUtxosRequest {
        address: ADDRESS.to_string(),
        filter: None,
    })
    .unwrap();

    assert_eq!(res.tip_height, 6);
    assert_eq!(&res.tip_block_hash, a[6].block_hash().as_bytes());

    // With two confirmations the tip is expected to be a[3].
    let res = get_utxos(GetUtxosRequest {
        address: ADDRESS.to_string(),
        filter: Some(UtxosFilter::MinConfirmations(2)),
    })
    .unwrap();

    assert_eq!(res.tip_height, 3);
    assert_eq!(&res.tip_block_hash, a[3].block_hash().as_bytes());
}

fn ingest_blocks<'a>(blocks: impl Iterator<Item = &'a Block>) {
    // Map the blocks into responses that are given to the hearbeat.
    let responses: Vec<_> = blocks
        .map(|block| {
            let mut block_bytes = vec![];
            Block::consensus_encode(block, &mut block_bytes).unwrap();
            GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
                GetSuccessorsCompleteResponse {
                    blocks: vec![block_bytes],
                    next: vec![],
                },
            ))
        })
        .collect();

    let responses_len = responses.len();

    set_successors_responses(responses);

    // Run the heartbeat until we process all the blocks.
    loop {
        block_on(async { heartbeat().await });

        if crate::runtime::GET_SUCCESSORS_RESPONSES_INDEX.with(|i| *i.borrow()) > responses_len {
            break;
        }
    }
}
