fn main() {
    let msg = lc_chain::GENESIS_MSG.trim();
    assert!(
        msg.starts_with("Last-change genesis."),
        "refusing to mine: founding text missing"
    );
    let b = lc_chain::mine_genesis(lc_chain::GENESIS_BITS);
    println!("{}", serde_json::to_string_pretty(&b).unwrap());
    eprintln!("HASH {}", b.header.hash());
    eprintln!("NONCE {}", b.header.nonce);
    eprintln!("WORK {}", b.header.work_bits());
}
