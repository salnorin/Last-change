//! circuit-pow: a small, bounded, deterministic random-program proof-of-work.
//!
//! seed -> Program (256..=512 integer gates incl. forward-only branches and
//! scratchpad loads/stores) ; nonce -> run Program -> 64-byte state ->
//! multi-hash chain (SHA-256, SHA-512, SHA3-256, BLAKE3; order permuted from
//! the seed) -> 32-byte digest. Work = leading zero bits of the digest.
//!
//! Integer-only (wrapping) arithmetic and forward-only branches make it
//! deterministic across CPUs and guarantee termination. Security of the hash
//! chain is roughly that of its weakest member; the chain adds diversity,
//! not strength. In Last Change this is ONLY an anti-spam stamp.

use sha2::{Digest, Sha256, Sha512};
use sha3::Sha3_256;

pub const MIN_GATES: usize = 256;
pub const MAX_GATES: usize = 512;
pub const REGS: usize = 8;
/// Scratchpad words (u64): 2048 * 8 = 16 KiB.
pub const SCRATCH_WORDS: usize = 2048;
pub const PASSES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Add(u8, u8, u8),
    Mul(u8, u8, u8),
    Xor(u8, u8, u8),
    Rot(u8, u8, u32),
    AddImm(u8, u64),
    Load { dst: u8, addr: u8 },
    Store { src: u8, addr: u8 },
    /// If (reg & mask) == 0, skip the next `skip` gates (forward only).
    BranchIfZero { reg: u8, mask: u64, skip: u16 },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub seed: [u8; 32],
    pub gates: Vec<Gate>,
    pub order: [u8; 4],
}

struct SplitMix(u64);
impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn seed_u64(seed: &[u8; 32], i: usize) -> u64 {
    u64::from_le_bytes(seed[i * 8..i * 8 + 8].try_into().unwrap())
}

impl Program {
    pub fn generate(seed: [u8; 32]) -> Self {
        let mut rng = SplitMix(seed_u64(&seed, 0) ^ seed_u64(&seed, 1).rotate_left(17));
        let n = MIN_GATES + (rng.next() as usize % (MAX_GATES - MIN_GATES + 1));
        let r = |x: u64| (x % REGS as u64) as u8;
        let gates = (0..n).map(|_| {
            let a = rng.next();
            let (d, s1, s2) = (r(a >> 8), r(a >> 16), r(a >> 24));
            match a % 8 {
                0 => Gate::Add(d, s1, s2),
                1 => Gate::Mul(d, s1, s2),
                2 => Gate::Xor(d, s1, s2),
                3 => Gate::Rot(d, s1, ((a >> 32) % 63 + 1) as u32),
                4 => Gate::AddImm(d, rng.next()),
                5 => Gate::Load { dst: d, addr: s1 },
                6 => Gate::Store { src: d, addr: s1 },
                _ => Gate::BranchIfZero { reg: d, mask: 1 << ((a >> 32) % 8), skip: ((a >> 40) % 8 + 1) as u16 },
            }
        }).collect();
        // Fisher-Yates permutation of the 4 hash functions.
        let mut order = [0u8, 1, 2, 3];
        for i in (1..4).rev() { order.swap(i, (rng.next() % (i as u64 + 1)) as usize); }
        Program { seed, gates, order }
    }

    /// Execute for `nonce`. `scratch` is reused across calls (no allocation).
    pub fn run(&self, nonce: u64, scratch: &mut [u64; SCRATCH_WORDS]) -> [u8; 64] {
        let mut rng = SplitMix(seed_u64(&self.seed, 2) ^ nonce);
        for w in scratch.iter_mut() { *w = rng.next(); }
        let mut reg = [0u64; REGS];
        for (i, x) in reg.iter_mut().enumerate() { *x = seed_u64(&self.seed, i % 4) ^ nonce.rotate_left(i as u32 * 8); }
        let mask = SCRATCH_WORDS as u64 - 1;
        for _ in 0..PASSES {
            let mut pc = 0;
            while pc < self.gates.len() {
                match self.gates[pc] {
                    Gate::Add(d, a, b) => reg[d as usize] = reg[a as usize].wrapping_add(reg[b as usize]),
                    Gate::Mul(d, a, b) => reg[d as usize] = reg[a as usize].wrapping_mul(reg[b as usize] | 1),
                    Gate::Xor(d, a, b) => reg[d as usize] = reg[a as usize] ^ reg[b as usize] ^ 0x5bd1_e995,
                    Gate::Rot(d, a, k) => reg[d as usize] = reg[a as usize].rotate_left(k),
                    Gate::AddImm(d, k) => reg[d as usize] = reg[d as usize].wrapping_add(k),
                    Gate::Load { dst, addr } => reg[dst as usize] ^= scratch[(reg[addr as usize] & mask) as usize],
                    Gate::Store { src, addr } => scratch[(reg[addr as usize] & mask) as usize] = reg[src as usize],
                    Gate::BranchIfZero { reg: r, mask: m, skip } => if reg[r as usize] & m == 0 { pc += skip as usize; },
                }
                pc += 1;
            }
        }
        let mut out = [0u8; 64];
        for (i, x) in reg.iter().enumerate() { out[i * 8..i * 8 + 8].copy_from_slice(&x.to_le_bytes()); }
        // fold part of the scratchpad in so stores matter
        let mut acc = 0u64;
        for (i, w) in scratch.iter().enumerate().step_by(97) { acc = acc.rotate_left(5) ^ w ^ i as u64; }
        out[0..8].iter_mut().zip(acc.to_le_bytes()).for_each(|(o, a)| *o ^= a);
        out
    }

    pub fn hash(&self, nonce: u64, scratch: &mut [u64; SCRATCH_WORDS]) -> [u8; 32] {
        multi_hash(&self.order, &self.run(nonce, scratch))
    }
}

/// Chain the four hash functions in `order`. Output 32 bytes.
pub fn multi_hash(order: &[u8; 4], data: &[u8]) -> [u8; 32] {
    let mut cur = data.to_vec();
    for f in order {
        cur = match f {
            0 => Sha256::digest(&cur).to_vec(),
            1 => Sha512::digest(&cur).to_vec(),
            2 => Sha3_256::digest(&cur).to_vec(),
            _ => blake3::hash(&cur).as_bytes().to_vec(),
        };
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&cur[..32]);
    out
}

pub fn leading_zero_bits(h: &[u8]) -> u32 {
    let mut n = 0;
    for b in h { if *b == 0 { n += 8 } else { n += b.leading_zeros(); break; } }
    n
}

/// Derive a seed from arbitrary context bytes.
pub fn seed_from(ctx: &[u8]) -> [u8; 32] { Sha256::digest(ctx).into() }

/// Find the first nonce with at least `bits` leading zero bits.
pub fn solve(seed: [u8; 32], bits: u32) -> u64 {
    let p = Program::generate(seed);
    let mut s = Box::new([0u64; SCRATCH_WORDS]);
    (0u64..).find(|n| leading_zero_bits(&p.hash(*n, &mut s)) >= bits).unwrap()
}

/// Work bits achieved by `nonce` for `seed`.
pub fn work_bits(seed: [u8; 32], nonce: u64) -> u32 {
    let mut s = Box::new([0u64; SCRATCH_WORDS]);
    leading_zero_bits(&Program::generate(seed).hash(nonce, &mut s))
}

pub fn verify(seed: [u8; 32], nonce: u64, bits: u32) -> bool { work_bits(seed, nonce) >= bits }

/// LWMA (linearly weighted moving average) next difficulty.
/// `timestamps` and `difficulties` cover the last N+1 / N blocks, oldest first.
pub fn lwma_next(timestamps: &[u64], difficulties: &[u128], target_secs: u64) -> u128 {
    let n = difficulties.len().min(timestamps.len().saturating_sub(1));
    if n == 0 { return difficulties.last().copied().unwrap_or(1).max(1); }
    let t = target_secs as i128;
    let ts = &timestamps[timestamps.len() - n - 1..];
    let ds = &difficulties[difficulties.len() - n..];
    let (mut weighted, mut sum_d) = (0i128, 0i128);
    for i in 1..=n {
        let solve = (ts[i] as i128 - ts[i - 1] as i128).clamp(-6 * t, 6 * t);
        weighted += solve * i as i128;
        sum_d += ds[i - 1] as i128;
    }
    let k = (n * (n + 1) / 2) as i128;
    let weighted = weighted.max(k * t / 10); // guard against tiny/negative sums
    let next = sum_d * t * k / (n as i128 * weighted);
    next.max(1) as u128
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

    #[test]
    fn program_bounded_and_deterministic() {
        for i in 0..50u8 {
            let p = Program::generate([i; 32]);
            assert!((MIN_GATES..=MAX_GATES).contains(&p.gates.len()));
            let mut o = p.order; o.sort(); assert_eq!(o, [0, 1, 2, 3]);
            assert_eq!(p.gates, Program::generate([i; 32]).gates);
        }
    }
    #[test]
    fn order_permutes_with_seed() {
        let orders: std::collections::HashSet<_> = (0..64u8).map(|i| Program::generate([i; 32]).order).collect();
        assert!(orders.len() > 4);
    }
    /// Pinned vector: must be identical on every platform (x86_64, aarch64, ...).
    #[test]
    fn cross_platform_vector() {
        let p = Program::generate(seed_from(b"lastchange-test-vector"));
        let mut s = Box::new([0u64; SCRATCH_WORDS]);
        assert_eq!(hex(&p.hash(42, &mut s)), PINNED);
    }
    const PINNED: &str = include_str!("../vector.txt");
    #[test]
    fn solve_verify() {
        let seed = seed_from(b"x");
        let n = solve(seed, 8);
        assert!(verify(seed, n, 8));
        assert!(work_bits(seed, n) >= 8);
    }
    #[test]
    fn lwma_responds() {
        let ts: Vec<u64> = (0..=60).map(|i| i * 120).collect();
        let ds = vec![1000u128; 60];
        let d = lwma_next(&ts, &ds, 120);
        assert!((990..=1010).contains(&d), "{d}");
        let fast: Vec<u64> = (0..=60).map(|i| i * 60).collect();
        assert!(lwma_next(&fast, &ds, 120) > 1500);
        let slow: Vec<u64> = (0..=60).map(|i| i * 240).collect();
        assert!(lwma_next(&slow, &ds, 120) < 700);
    }
}
