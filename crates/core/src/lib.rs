//! Core types for Last Change: pseudonymous identities, listings, deal receipts,
//! ratings, reports and signed envelopes with an anti-spam proof-of-work stamp.
//! There is deliberately NO currency, payment or escrow type in this crate.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod filter;

/// Default anti-spam stamp difficulty (leading zero bits). Cheap: ~65k hashes.
pub const DEFAULT_STAMP_BITS: u32 = 16;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("bad signature")]
    BadSignature,
    #[error("bad key")]
    BadKey,
    #[error("insufficient anti-spam stamp")]
    WeakStamp,
    #[error("invalid payload: {0}")]
    Invalid(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// A locally generated identity. Nothing but a keypair.
pub struct Identity {
    key: SigningKey,
}

impl Identity {
    pub fn generate() -> Self {
        Self { key: SigningKey::generate(&mut rand::rngs::OsRng) }
    }
    pub fn from_secret_bytes(b: &[u8; 32]) -> Self {
        Self { key: SigningKey::from_bytes(b) }
    }
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.key.to_bytes()
    }
    pub fn public_hex(&self) -> String {
        hex::encode(self.key.verifying_key().to_bytes())
    }
    pub fn pseudonym(&self) -> String {
        pseudonym_of(&self.public_hex())
    }
    /// Sign a payload, mining a small anti-spam stamp first.
    pub fn seal(&self, payload: &Payload, stamp_bits: u32) -> Envelope {
        let body = serde_json::to_string(payload).expect("serialize");
        let author = self.public_hex();
        let nonce = find_stamp(&author, &body, stamp_bits);
        let msg = signing_bytes(&author, &body, nonce);
        let sig = self.key.sign(&msg);
        Envelope { author, body, nonce, sig: hex::encode(sig.to_bytes()) }
    }
}

/// Pseudonym = first 20 bytes of sha256(pubkey), hex.
pub fn pseudonym_of(pub_hex: &str) -> String {
    let bytes = hex::decode(pub_hex).unwrap_or_default();
    hex::encode(&Sha256::digest(bytes)[..20])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListingKind {
    Offer,
    Request,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub kind: ListingKind,
    pub title: String,
    pub desc: String,
    /// Free text only ("negotiable", "trade for a bike"). Not a payment.
    pub price_hint: String,
    pub category: String,
    /// Unix seconds.
    pub expiry: u64,
    /// Content hashes / links to off-ledger images.
    pub image_hashes: Vec<String>,
    pub created: u64,
}

/// Signed by the SELLER: confirms a deal with `buyer` happened off-network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DealReceipt {
    pub listing_id: String,
    pub buyer: String, // buyer pubkey hex
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rating {
    pub listing_id: String,
    pub seller: String, // seller pubkey hex
    pub receipt_id: String,
    pub score: u8,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub target_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    Listing(Listing),
    DealReceipt(DealReceipt),
    Rating(Rating),
    Report(Report),
}

impl Payload {
    pub fn validate(&self) -> Result<(), CoreError> {
        let bad = |s: &str| Err(CoreError::Invalid(s.into()));
        match self {
            Payload::Listing(l) => {
                if l.title.trim().is_empty() || l.title.len() > 200 { return bad("title length"); }
                if l.desc.len() > 4000 { return bad("desc too long"); }
                if l.image_hashes.len() > 8 { return bad("too many images"); }
                if filter::is_forbidden(&format!("{} {} {}", l.title, l.desc, l.category)) {
                    return bad("content filter");
                }
            }
            Payload::Rating(r) => {
                if !(1..=5).contains(&r.score) { return bad("score must be 1-5"); }
                if r.comment.len() > 1000 { return bad("comment too long"); }
            }
            Payload::Report(r) => if r.reason.len() > 500 { return bad("reason too long"); },
            Payload::DealReceipt(_) => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub author: String,
    pub body: String,
    pub nonce: u64,
    pub sig: String,
}

fn signing_bytes(author: &str, body: &str, nonce: u64) -> Vec<u8> {
    format!("lastchange-v1|{author}|{nonce}|{body}").into_bytes()
}

fn leading_zero_bits(h: &[u8]) -> u32 {
    let mut n = 0;
    for b in h {
        if *b == 0 { n += 8 } else { n += b.leading_zeros(); break; }
    }
    n
}

fn stamp_bits(author: &str, body: &str, nonce: u64) -> u32 {
    leading_zero_bits(&Sha256::digest(signing_bytes(author, body, nonce)))
}

fn find_stamp(author: &str, body: &str, bits: u32) -> u64 {
    (0u64..).find(|n| stamp_bits(author, body, *n) >= bits).unwrap()
}

impl Envelope {
    pub fn id(&self) -> String {
        sha256_hex(&signing_bytes(&self.author, &self.body, self.nonce))
    }
    pub fn author_pseudonym(&self) -> String {
        pseudonym_of(&self.author)
    }
    pub fn payload(&self) -> Result<Payload, CoreError> {
        Ok(serde_json::from_str(&self.body)?)
    }
    /// Verify signature, stamp and payload rules.
    pub fn verify(&self, min_stamp_bits: u32) -> Result<Payload, CoreError> {
        let pk: [u8; 32] = hex::decode(&self.author).ok().and_then(|v| v.try_into().ok()).ok_or(CoreError::BadKey)?;
        let vk = VerifyingKey::from_bytes(&pk).map_err(|_| CoreError::BadKey)?;
        let sig: [u8; 64] = hex::decode(&self.sig).ok().and_then(|v| v.try_into().ok()).ok_or(CoreError::BadSignature)?;
        vk.verify(&signing_bytes(&self.author, &self.body, self.nonce), &Signature::from_bytes(&sig))
            .map_err(|_| CoreError::BadSignature)?;
        if stamp_bits(&self.author, &self.body, self.nonce) < min_stamp_bits {
            return Err(CoreError::WeakStamp);
        }
        let p = self.payload()?;
        p.validate()?;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn listing() -> Payload {
        Payload::Listing(Listing { kind: ListingKind::Offer, title: "Old bicycle".into(), desc: "works".into(),
            price_hint: "negotiable".into(), category: "bikes".into(), expiry: 0, image_hashes: vec![], created: 0 })
    }
    #[test]
    fn seal_and_verify() {
        let id = Identity::generate();
        let e = id.seal(&listing(), 8);
        assert_eq!(e.verify(8).unwrap(), listing());
        assert_eq!(e.author_pseudonym(), id.pseudonym());
        assert_eq!(id.pseudonym().len(), 40);
    }
    #[test]
    fn tamper_detected() {
        let mut e = Identity::generate().seal(&listing(), 4);
        e.body = e.body.replace("bicycle", "car");
        assert!(matches!(e.verify(0), Err(CoreError::BadSignature)));
    }
    #[test]
    fn weak_stamp_rejected() {
        let e = Identity::generate().seal(&listing(), 0);
        // Very unlikely that nonce 0 already satisfies 24 bits.
        if stamp_bits(&e.author, &e.body, e.nonce) < 24 { assert!(matches!(e.verify(24), Err(CoreError::WeakStamp))); }
    }
    #[test]
    fn bad_score_and_filter() {
        let r = Payload::Rating(Rating { listing_id: "x".into(), seller: "s".into(), receipt_id: "r".into(), score: 6, comment: "".into() });
        assert!(r.validate().is_err());
        let mut l = listing();
        if let Payload::Listing(ref mut x) = l { x.title = "cheap firearms".into(); }
        assert!(l.validate().is_err());
    }
    #[test]
    fn key_roundtrip() {
        let a = Identity::generate();
        let b = Identity::from_secret_bytes(&a.secret_bytes());
        assert_eq!(a.public_hex(), b.public_hex());
    }
}
