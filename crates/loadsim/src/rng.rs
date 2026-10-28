//! xoshiro256++ seeded through SplitMix64: small, fast, and identical on every platform. The seed
//! comes from JavaScript — WebAssembly gets no OS randomness here.

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            x ^ (x >> 31)
        };
        Rng { s: [next(), next(), next(), next()] }
    }

    pub fn next_u64(&mut self) -> u64 {
        let [s0, s1, s2, s3] = &mut self.s;
        let out = s0.wrapping_add(*s3).rotate_left(23).wrapping_add(*s0);
        let t = *s1 << 17;
        *s2 ^= *s0;
        *s3 ^= *s1;
        *s1 ^= *s2;
        *s0 ^= *s3;
        *s2 ^= t;
        *s3 = s3.rotate_left(45);
        out
    }

    /// Uniform in (0, 1] — never 0, so `ln` is always finite.
    pub fn unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) + 1) as f64 / (1u64 << 53) as f64
    }

    /// Exponentially distributed with the given mean.
    pub fn exp(&mut self, mean: f64) -> f64 {
        -mean * self.unit().ln()
    }

    /// Standard normal (Box–Muller).
    pub fn normal(&mut self) -> f64 {
        (-2.0 * self.unit().ln()).sqrt() * (core::f64::consts::TAU * self.unit()).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let (mut a, mut b, mut c) = (Rng::new(7), Rng::new(7), Rng::new(8));
        for _ in 0..100 {
            let x = a.next_u64();
            assert_eq!(x, b.next_u64());
            assert_ne!(x, c.next_u64());
        }
    }

    #[test]
    fn moments() {
        let mut r = Rng::new(1);
        let n = 200_000;
        let (mut u, mut e, mut z, mut z2) = (0.0, 0.0, 0.0, 0.0);
        for _ in 0..n {
            let x = r.unit();
            assert!(x > 0.0 && x <= 1.0);
            u += x;
            e += r.exp(20.0);
            let g = r.normal();
            z += g;
            z2 += g * g;
        }
        let n = n as f64;
        assert!((u / n - 0.5).abs() < 0.005);
        assert!((e / n - 20.0).abs() < 0.3, "exponential mean {}", e / n);
        assert!((z / n).abs() < 0.01);
        assert!((z2 / n - 1.0).abs() < 0.02);
    }
}
