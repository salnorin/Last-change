use lc_core::*;
use std::time::Instant;
fn t<F: FnMut()>(n: u32, mut f: F) -> f64 {
    let s = Instant::now();
    for _ in 0..n {
        f()
    }
    s.elapsed().as_secs_f64() * 1e6 / n as f64
}
fn main() {
    let id = Identity::generate();
    let p = Payload::Report(Report {
        target_id: "x".into(),
        reason: "bench".into(),
    });
    let e = id.sign(&p);
    println!(
        "ed25519 sig 64 B, pubkey 32 B; ML-DSA-65 sig {} B, pubkey {} B",
        e.pq_sig.len() / 2,
        e.pq_pub.len() / 2
    );
    println!(
        "envelope JSON {} B (hex-encoded)",
        serde_json::to_string(&e).unwrap().len()
    );
    println!(
        "hybrid sign:   {:.0} us",
        t(200, || {
            std::hint::black_box(id.sign(&p));
        })
    );
    println!(
        "hybrid verify: {:.0} us",
        t(200, || {
            std::hint::black_box(e.verify_signatures().unwrap());
        })
    );
    let k = id.kem_public();
    let s = channel::seal_to(&k, b"hello", b"").unwrap();
    println!(
        "ML-KEM-768 ct {} B, ek {} B",
        s.kem_ct.len() / 2,
        k.mlkem768.len() / 2
    );
    println!(
        "hybrid seal:   {:.0} us",
        t(200, || {
            std::hint::black_box(channel::seal_to(&k, b"hello", b"").unwrap());
        })
    );
    println!(
        "hybrid open:   {:.0} us",
        t(200, || {
            std::hint::black_box(id.open(&s, b"").unwrap());
        })
    );
    println!(
        "keystore unlock (Argon2id 64MiB t=3): {:.0} ms",
        t(3, || {
            let ks = keystore::Keystore::encrypt(&id, "benchmark-pass").unwrap();
            std::hint::black_box(ks);
        }) / 1000.0
    );
}
