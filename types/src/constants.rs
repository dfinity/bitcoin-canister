use bitcoin::{util::uint::Uint256, Network};

// The size of an outpoint in bytes.
const OUTPOINT_TX_ID_SIZE: u32 = 32; // The size of the transaction ID.
const OUTPOINT_VOUT_SIZE: u32 = 4; // The size of a transaction's vout.
pub const OUTPOINT_SIZE: u32 = OUTPOINT_TX_ID_SIZE + OUTPOINT_VOUT_SIZE;

/// Bitcoin mainnet maximum target value
const BITCOIN_MAX_TARGET: Uint256 = Uint256([
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x00000000ffff0000,
]);

/// Bitcoin testnet maximum target value
const TESTNET_MAX_TARGET: Uint256 = Uint256([
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x00000000ffff0000,
]);

/// Bitcoin regtest maximum target value
const REGTEST_MAX_TARGET: Uint256 = Uint256([
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x7fffff0000000000,
]);

/// Bitcoin signet maximum target value
const SIGNET_MAX_TARGET: Uint256 = Uint256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x00000377ae000000u64,
]);

/// Returns the maximum difficulty target depending on the network
pub fn max_target(network: &Network) -> Uint256 {
    match network {
        Network::Bitcoin => BITCOIN_MAX_TARGET,
        Network::Testnet => TESTNET_MAX_TARGET,
        Network::Regtest => REGTEST_MAX_TARGET,
        Network::Signet => SIGNET_MAX_TARGET,
    }
}
