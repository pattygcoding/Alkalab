//! A tiny deterministic pseudo random number generator (xorshift32).
//!
//! The simulation deliberately avoids the `rand` crate: a hand rolled PRNG keeps
//! the wasm module small, is trivially portable to any target and - most
//! importantly - makes the whole simulation perfectly reproducible from a seed
//! so that unit tests can assert on exact behaviour.

/// xorshift32 PRNG. Fast (a handful of ALU ops), deterministic and good enough
/// for a cellular automaton where we only need "roughly 1 in 16" style rolls.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u32,
}

impl Rng {
    /// Creates a generator. A seed of `0` is remapped because xorshift32 would
    /// otherwise be stuck at zero forever.
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    /// Raw 32 bit output.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform value in `0..n`. Returns `0` for `n == 0` instead of panicking.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }

    /// `true` with probability `p / 255`. `0` is never true, `255` is always true.
    #[inline]
    pub fn chance(&mut self, p: u8) -> bool {
        p == 255 || (p != 0 && (self.next_u32() & 0xFF) < u32::from(p))
    }

    /// Fair coin flip.
    #[inline]
    pub fn flip(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// Inclusive range in `lo..=hi`.
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_seed() {
        let mut a = Rng::new(1234);
        let mut b = Rng::new(1234);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn zero_seed_is_remapped_and_keeps_flowing() {
        let mut r = Rng::new(0);
        let first = r.next_u32();
        assert_ne!(first, 0);
        assert_ne!(first, r.next_u32());
    }

    #[test]
    fn chance_bounds() {
        let mut r = Rng::new(42);
        for _ in 0..10_000 {
            assert!(!r.chance(0));
            assert!(r.chance(255));
        }
    }

    #[test]
    fn below_stays_in_range() {
        let mut r = Rng::new(7);
        for _ in 0..10_000 {
            assert!(r.below(1) == 0);
            assert!(r.below(6) < 6);
            assert_eq!(r.below(0), 0);
        }
    }

    #[test]
    fn chance_is_roughly_uniform() {
        let mut r = Rng::new(99);
        let mut hits = 0;
        const N: u32 = 100_000;
        for _ in 0..N {
            if r.chance(64) {
                hits += 1;
            }
        }
        // p = 64/255 = 0.251
        let ratio = f64::from(hits) / f64::from(N);
        assert!((0.23..0.27).contains(&ratio), "ratio was {ratio}");
    }

    #[test]
    fn range_i32_is_inclusive() {
        let mut r = Rng::new(5);
        for _ in 0..10_000 {
            let v = r.range_i32(-3, 3);
            assert!((-3..=3).contains(&v));
        }
        assert_eq!(r.range_i32(4, 4), 4);
        assert_eq!(r.range_i32(9, 2), 9);
    }
}
