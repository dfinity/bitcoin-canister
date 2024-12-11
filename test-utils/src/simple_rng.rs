use bitcoin::{
    secp256k1,
    secp256k1::{constants::SECRET_KEY_SIZE, Secp256k1, Signing},
};
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<SimpleRng> = RefCell::new(SimpleRng::new(0));
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

pub fn generate_keypair<C: Signing>(
    secp: &Secp256k1<C>,
) -> (secp256k1::SecretKey, secp256k1::PublicKey) {
    with_rng(|rng| {
        let mut data = [0u8; SECRET_KEY_SIZE];
        rng.fill_bytes(&mut data);
        let sk = secp256k1::SecretKey::from_slice(&data).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(secp, &sk);
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
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // XOR-Shift implementation
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
