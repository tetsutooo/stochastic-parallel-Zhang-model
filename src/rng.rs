#![allow(dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct FastRng {
    s: [u64; 4],
}

impl FastRng {
    #[inline]
    pub fn new(seed: u64) -> Self {
        let mut sm = SplitMix64 { state: seed };
        Self {
            s: [sm.next(), sm.next(), sm.next(), sm.next()],
        }
    }
    #[inline]
    pub fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self::new(nanos ^ 0xA5A5_5A5A_DEAD_BEEF)
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[0]
            .wrapping_add(self.s[3])
            .rotate_left(23)
            .wrapping_add(self.s[0]);

        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    #[inline(always)]
    pub fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    #[inline(always)]
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    #[inline(always)]
    pub fn gen_f64_range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.next_f64()
    }

    #[inline(always)]
    pub fn gen_f64_upto(&mut self, high: f64) -> f64 {
        high * self.next_f64()
    }

    #[inline(always)]
    pub fn gen_usize_range(&mut self, low: usize, high: usize) -> usize {
        let range = (high - low) as u64;
        low + lemire_bounded(self, range) as usize
    }
}

#[inline(always)]
fn lemire_bounded(rng: &mut FastRng, range: u64) -> u64 {
    let x = rng.next_u64();
    let mut m = (x as u128).wrapping_mul(range as u128);
    let mut l = m as u64;
    if l < range {
        let t = range.wrapping_neg() % range;
        while l < t {
            let x = rng.next_u64();
            m = (x as u128).wrapping_mul(range as u128);
            l = m as u64;
        }
    }
    (m >> 64) as u64
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[inline]
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_in_unit_interval() {
        let mut rng = FastRng::new(42);
        for _ in 0..10_000 {
            let x = rng.next_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn usize_range_within_bounds() {
        let mut rng = FastRng::new(7);
        for _ in 0..10_000 {
            let v = rng.gen_usize_range(10, 20);
            assert!((10..20).contains(&v));
        }
    }

    #[test]
    fn deterministic_for_same_seed() {
        let mut a = FastRng::new(123);
        let mut b = FastRng::new(123);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}
