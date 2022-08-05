use crate::types::Network;
use bitcoin::{secp256k1::rand::rngs::OsRng, secp256k1::Secp256k1, Address, PublicKey};

/// Generates a random P2PKH address.
pub fn random_p2pkh_address(network: Network) -> Address {
    let secp = Secp256k1::new();
    let mut rng = OsRng::new().unwrap();

    Address::p2pkh(
        &PublicKey::new(secp.generate_keypair(&mut rng).1),
        network.into(),
    )
}
