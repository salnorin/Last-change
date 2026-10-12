use circuit_pow::*;
use std::time::Instant;
fn main() {
    let p = Program::generate(seed_from(b"bench"));
    let mut s = Box::new([0u64; SCRATCH_WORDS]);
    let n = 20_000;
    let t = Instant::now();
    for i in 0..n {
        std::hint::black_box(p.hash(i, &mut s));
    }
    let per = t.elapsed().as_secs_f64() / n as f64;
    println!(
        "circuit-pow: {:.1} us/hash, {:.0} hashes/s; 16-bit stamp ~{:.2} s, 12-bit ~{:.2} s",
        per * 1e6,
        1.0 / per,
        per * 65536.0,
        per * 4096.0
    );
    let t = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(work_bits(seed_from(b"v"), 1));
    }
    println!(
        "verify (generate+1 hash): {:.1} us",
        t.elapsed().as_secs_f64() * 1e3
    );
}
