//! Zero-value TESTNET genesis and local tip state.
//! Coinbase value is ALWAYS 0. No balances, rewards, or transfers.

use circuit_pow::{leading_zero_bits, Program, SCRATCH_WORDS};
use lc_core::{verify_bytes, Identity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Founding message (exact owner text).
pub const GENESIS_MSG: &str = include_str!("../FOUNDING.txt");
pub const GENESIS_TIME: u64 = 1_791_764_488; // 2026-10-12T00:21:28Z (BTC tip 971010 observed then)
pub const GENESIS_HEADLINE: &str =
    "btc_height=971010 tip=000000000000000000018dd13274c1a60d382ad2a10e4b6aa175c5cb3f1035d1";
pub const GENESIS_BITS: u32 = 16;
pub const DOMAIN: &str = "lastchange-testnet-block-v1";
/// HARDCODED. Asserted in tests.
pub const COINBASE_VALUE: u64 = 0;

#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("bad pow")]
    BadPow,
    #[error("bad signature")]
    BadSig,
    #[error("coinbase")]
    BadCoinbase,
    #[error("genesis outputs are unspendable")]
    GenesisUnspendable,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub version: u32,
    pub prev: String,
    pub merkle_root: String,
    pub time: u64,
    pub bits: u32,
    pub nonce: u64,
    pub circuit_seed: String,
    pub miner_ed: String,
    pub miner_pq: String,
    pub sig: String,
    pub pq_sig: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coinbase {
    /// Spendable coinbase value: ALWAYS 0.
    pub value: u64,
    pub flag: String,
    pub msg: String,
    /// Genesis only: symbolic output, provably unspendable (nothing-up-my-sleeve script).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burn: Option<BurnOutput>,
}

/// A symbolic, permanently unspendable output. The script is `burn:` + sha256(genesis message),
/// which is not a public key: no one has a matching private key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BurnOutput {
    pub amount_est: u64,
    pub script: String,
}

/// Genesis burn amount (symbolic, unspendable by consensus).
pub const GENESIS_BURN_EST: u64 = 50;

pub fn genesis_burn_script() -> String {
    format!(
        "burn:{}",
        hex::encode(Sha256::digest(msg_trimmed().as_bytes()))
    )
}

/// Consensus rule: outputs created by the genesis block are never added to any
/// spendable set. Any future transaction referencing them is invalid.
pub fn validate_spend_ref(referenced_block_hash: &str) -> Result<(), ChainError> {
    if referenced_block_hash == genesis_hash() {
        return Err(ChainError::GenesisUnspendable);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub header: Header,
    pub coinbase: Coinbase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitView {
    pub seed: String,
    pub gates: usize,
    pub hash_order: Vec<String>,
    pub bits: u32,
    pub work_bits: u32,
}

const HASH_NAMES: [&str; 4] = ["sha256", "sha512", "sha3-256", "blake3"];

impl Header {
    pub fn preimage(&self) -> Vec<u8> {
        format!(
            "v{}|{}|{}|{}|{}|{}|{}|{}",
            self.version,
            self.prev,
            self.merkle_root,
            self.time,
            self.bits,
            self.circuit_seed,
            self.miner_ed,
            self.miner_pq
        )
        .into_bytes()
    }
    pub fn hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.preimage());
        h.update(self.nonce.to_le_bytes());
        hex::encode(h.finalize())
    }
    fn pow_seed(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"lastchange-testnet-pow-v1|");
        h.update(self.preimage());
        h.finalize().into()
    }
    pub fn work_bits(&self) -> u32 {
        let mut s = Box::new([0u64; SCRATCH_WORDS]);
        leading_zero_bits(&Program::generate(self.pow_seed()).hash(self.nonce, &mut s))
    }
    pub fn circuit_view(&self) -> CircuitView {
        let prog = Program::generate(self.pow_seed());
        CircuitView {
            seed: hex::encode(self.pow_seed()),
            gates: prog.gates.len(),
            hash_order: prog
                .order
                .iter()
                .map(|i| HASH_NAMES[*i as usize].to_string())
                .collect(),
            bits: self.bits,
            work_bits: self.work_bits(),
        }
    }
    pub fn verify_pow(&self) -> Result<(), ChainError> {
        if self.work_bits() < self.bits {
            return Err(ChainError::BadPow);
        }
        Ok(())
    }
    pub fn verify_sig(&self) -> Result<(), ChainError> {
        verify_bytes(
            DOMAIN,
            &self.preimage(),
            &self.miner_ed,
            &self.miner_pq,
            &self.sig,
            &self.pq_sig,
        )
        .map_err(|_| ChainError::BadSig)
    }
}

impl Block {
    pub fn validate(&self) -> Result<(), ChainError> {
        if self.coinbase.value != COINBASE_VALUE {
            return Err(ChainError::BadCoinbase);
        }
        if !self.coinbase.flag.contains("value: disabled") {
            return Err(ChainError::BadCoinbase);
        }
        let is_genesis = self.header.prev == "0".repeat(64);
        match (&self.coinbase.burn, is_genesis) {
            (None, false) => {}
            (Some(b), true)
                if b.amount_est == GENESIS_BURN_EST && b.script == genesis_burn_script() => {}
            _ => return Err(ChainError::BadCoinbase),
        }
        self.header.verify_pow()?;
        self.header.verify_sig()?;
        Ok(())
    }
}

fn msg_trimmed() -> &'static str {
    GENESIS_MSG.trim()
}

/// Mine genesis once (build tool). Deterministic miner seed [7;32].
pub fn mine_genesis(bits: u32) -> Block {
    let id = Identity::from_master(&[7u8; 32]);
    let full_msg = format!("{} | {}", msg_trimmed(), GENESIS_HEADLINE);
    let merkle = hex::encode(Sha256::digest(format!("coinbase|0|{full_msg}").as_bytes()));
    let mut hdr = Header {
        version: 1,
        prev: "0".repeat(64),
        merkle_root: merkle,
        time: GENESIS_TIME,
        bits,
        nonce: 0,
        circuit_seed: hex::encode(Sha256::digest(b"lastchange-genesis-seed")),
        miner_ed: id.public_hex(),
        miner_pq: id.pq_public_hex(),
        sig: String::new(),
        pq_sig: String::new(),
    };
    let (sig, pq) = id.sign_bytes(DOMAIN, &hdr.preimage());
    hdr.sig = sig;
    hdr.pq_sig = pq;
    let pow_seed = hdr.pow_seed();
    let prog = Program::generate(pow_seed);
    let mut scratch = Box::new([0u64; SCRATCH_WORDS]);
    hdr.nonce = (0u64..)
        .find(|&n| leading_zero_bits(&prog.hash(n, &mut scratch)) >= bits)
        .unwrap();
    Block {
        header: hdr,
        coinbase: Coinbase {
            value: COINBASE_VALUE,
            flag: "value: disabled (design-only)".into(),
            msg: full_msg,
            burn: Some(BurnOutput {
                amount_est: GENESIS_BURN_EST,
                script: genesis_burn_script(),
            }),
        },
    }
}

pub fn genesis() -> Block {
    let b: Block = serde_json::from_str(include_str!("../genesis.json")).expect("genesis.json");
    b
}

pub fn genesis_hash() -> String {
    include_str!("../genesis.hash").trim().to_string()
}

/// Local chain state: tip is genesis until more blocks exist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChain {
    pub genesis_hash: String,
    pub tip_hash: String,
    pub height: u64,
    pub blocks: Vec<Block>,
}

impl LocalChain {
    pub fn from_genesis() -> Self {
        let g = genesis();
        let h = g.header.hash();
        Self {
            genesis_hash: h.clone(),
            tip_hash: h,
            height: 0,
            blocks: vec![g],
        }
    }
    pub fn save(&self, dir: &Path) -> Result<(), ChainError> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("chain.json"), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
    pub fn load_or_init(dir: &Path) -> Result<Self, ChainError> {
        let p = dir.join("chain.json");
        if p.exists() {
            Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?)
        } else {
            let c = Self::from_genesis();
            c.save(dir)?;
            Ok(c)
        }
    }
    pub fn block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.header.hash() == hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn founding_text_exact() {
        assert_eq!(
            msg_trimmed(),
            "Last-change genesis. Buying and selling here is not bound to any third party: anyone can sell anything to anyone, and no one else needs to know who they are. For Satoshi Nakamoto, who stepped out of the light so that no one would need to be trusted, and showed that power shared by everyone belongs to no one. 50 EST locked forever, spendable by no one."
        );
    }
    #[test]
    fn coinbase_zero_and_genesis_valid() {
        assert_eq!(COINBASE_VALUE, 0);
        let g = genesis();
        g.validate().unwrap();
        assert_eq!(g.coinbase.value, 0);
        assert!(g.coinbase.msg.contains(msg_trimmed()));
        assert_eq!(g.header.hash(), genesis_hash());
        assert_eq!(g.header.prev, "0".repeat(64));
        let mut bad = g.clone();
        bad.coinbase.value = 1;
        assert!(matches!(bad.validate(), Err(ChainError::BadCoinbase)));
    }
    #[test]
    fn genesis_burn_is_unspendable() {
        let g = genesis();
        let b = g.coinbase.burn.as_ref().expect("burn output");
        assert_eq!(b.amount_est, 50);
        assert_eq!(b.script, genesis_burn_script());
        assert!(g
            .coinbase
            .msg
            .contains("50 EST locked forever, spendable by no one."));
        // any reference to genesis outputs is invalid
        assert!(matches!(
            validate_spend_ref(&genesis_hash()),
            Err(ChainError::GenesisUnspendable)
        ));
        assert!(validate_spend_ref(&"ab".repeat(32)).is_ok());
        // a burn on a non-genesis block, or a different amount, is rejected
        let mut bad = g.clone();
        bad.coinbase.burn.as_mut().unwrap().amount_est = 51;
        assert!(matches!(bad.validate(), Err(ChainError::BadCoinbase)));
    }
    #[test]
    fn local_chain_height_zero() {
        let c = LocalChain::from_genesis();
        assert_eq!(c.height, 0);
        assert_eq!(c.tip_hash, genesis_hash());
    }
}
