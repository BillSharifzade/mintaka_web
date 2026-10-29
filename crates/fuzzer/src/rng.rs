//! xorshift64* — small, fast and deterministic, so every run can be replayed from its seed.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state would stay zero forever; mix the seed so nearby seeds diverge quickly.
        let mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
        Rng(if mixed == 0 { 1 } else { mixed })
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n`. `n` must be non-zero and below 2³².
    #[inline(always)]
    pub fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        (((self.next_u64() >> 32) * n as u64) >> 32) as usize
    }

    #[inline(always)]
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// True with probability `num / den`.
    #[inline(always)]
    pub fn chance(&mut self, num: usize, den: usize) -> bool {
        self.below(den) < num
    }
}
