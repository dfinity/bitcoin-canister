//! SimpleRng
//!
//! This module provides a lightweight implementation of a simple random number generator (SimpleRng).
//! The purpose of this implementation is to avoid a dependency on the `rand` crate, which relies on
//! the `getrandom` crate for randomness. Starting with version `0.2.0`, `getrandom` with the `"js"`
//! feature expects a JavaScript and Node.js environment. For more details, see the documentation:
//! <https://docs.rs/getrandom/0.2.0/getrandom/index.html#webassembly-support>.
//!
//! When compiled for the `wasm32-unknown-unknown` target, this expectation causes runtime errors
//! because the Internet Computer (IC) production environment does not support Node.js modules.
//!
//! As a temporary solution, SimpleRng is used in place of the `rand` crate. This RNG is not cryptographically
//! secure but is sufficient for the `ic-btc-test-utils` library, which is intended for testing purposes only.
//!
//! Note: This implementation should *NOT* be used in production or for applications requiring cryptographic security.

use bitcoin::secp256k1::{constants::SECRET_KEY_SIZE, PublicKey, Secp256k1, SecretKey, Signing};
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<SimpleRng> = RefCell::new(SimpleRng::new(37));
}

fn with_rng<F, R>(f: F) -> R
where
    F: FnOnce(&mut SimpleRng) -> R,
{
    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        f(&mut rng)
    })
}

pub fn generate_keypair<C: Signing>(secp: &Secp256k1<C>) -> (SecretKey, PublicKey) {
    with_rng(|rng| {
        let mut data = [0u8; SECRET_KEY_SIZE];
        rng.fill_bytes(&mut data);
        let sk = SecretKey::from_slice(&data).unwrap();
        let pk = PublicKey::from_secret_key(secp, &sk);
        (sk, pk)
    })
}

pub fn fill_bytes(dest: &mut [u8]) {
    with_rng(|rng| rng.fill_bytes(dest))
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xC0FFEE } else { seed }, // Seed value must not be zero.
        }
    }

    fn next_u64(&mut self) -> u64 {
        // XOR-Shift implementation, see https://en.wikipedia.org/wiki/Xorshift
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut chunks = dest.chunks_exact_mut(8);
        for chunk in &mut chunks {
            let value = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&value);
        }
        let remainder = chunks.into_remainder();
        if !remainder.is_empty() {
            let value = self.next_u64().to_le_bytes();
            remainder.copy_from_slice(&value[..remainder.len()]);
        }
    }
}
