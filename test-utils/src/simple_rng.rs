pub struct SimpleRng {
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

    pub fn random_32_bytes(&mut self) -> [u8; 32] {
        let mut ret = [0u8; 32];
        self.fill_bytes(&mut ret);
        ret
    }
}
