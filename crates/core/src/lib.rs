//! Core types for Last Change: pseudonymous identities, listings, deal receipts,
//! ratings, reports and signed envelopes with an anti-spam proof-of-work stamp.
//! There is deliberately NO currency, payment or escrow type in this crate.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ml_dsa::MlDsa65;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod channel;
pub mod filter;
pub mod keystore;

/// Default anti-spam stamp difficulty (leading zero bits).
pub const DEFAULT_STAMP_BITS: u32 = 16;
/// ML-DSA domain-separation context.
const PQ_CTX: &[u8] = b"lastchange-v2";

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("bad signature")]
    BadSignature,
    #[error("bad post-quantum signature")]
    BadPqSignature,
    #[error("bad key")]
    BadKey,
    #[error("insufficient anti-spam stamp")]
    WeakStamp,
    #[error("invalid payload: {0}")]
    Invalid(String),
    #[error("crypto: {0}")]
    Crypto(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Derive a 32-byte sub-key from the master seed (HKDF-SHA256).
pub(crate) fn derive(master: &[u8; 32], label: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    hkdf::Hkdf::<Sha256>::new(Some(b"lastchange-keys-v1"), master).expand(label.as_bytes(), &mut out).expect("hkdf");
    out
}

/// A locally generated hybrid identity: Ed25519 + ML-DSA-65 (FIPS 204), plus
/// X25519 + ML-KEM-768 keys for encrypted deal messages. All derived from one
/// 32-byte master seed. No other data.
pub struct Identity {
    master: [u8; 32],
    ed: SigningKey,
    pq: ml_dsa::SigningKey<MlDsa65>,
}

impl Identity {
    pub fn generate() -> Self {
        let mut m = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut m);
        Self::from_master(&m)
    }
    pub fn from_master(m: &[u8; 32]) -> Self {
        let ed = SigningKey::from_bytes(&derive(m, "ed25519"));
        let seed = derive(m, "ml-dsa-65");
        let pq = ml_dsa::SigningKey::<MlDsa65>::from_seed(&seed.into());
        Self { master: *m, ed, pq }
    }
    pub fn master_bytes(&self) -> [u8; 32] { self.master }
    pub fn public_hex(&self) -> String { hex::encode(self.ed.verifying_key().to_bytes()) }
    pub fn pq_public_hex(&self) -> String { hex::encode(self.pq.expanded_key().verifying_key().encode().as_slice()) }
    /// Pseudonym = first 20 bytes of sha256(ed25519_pub || ml_dsa_pub).
    pub fn pseudonym(&self) -> String { pseudonym_of(&self.public_hex(), &self.pq_public_hex()) }

    /// Client side: sign (both algorithms) without a stamp (nonce = 0).
    pub fn sign(&self, payload: &Payload) -> Envelope {
        let body = serde_json::to_string(payload).expect("serialize");
        let (author, pq_pub) = (self.public_hex(), self.pq_public_hex());
        let msg = signing_bytes(&author, &pq_pub, &body);
        let sig = hex::encode(self.ed.sign(&msg).to_bytes());
        let pq_sig = self.pq.expanded_key().sign_deterministic(&msg, PQ_CTX).expect("ml-dsa sign");
        Envelope { author, pq_pub, body, sig, pq_sig: hex::encode(pq_sig.encode().as_slice()), nonce: 0 }
    }
    /// Sign and stamp locally (desktop/server nodes).
    pub fn seal(&self, payload: &Payload, stamp_bits: u32) -> Envelope {
        let mut e = self.sign(payload);
        e.stamp(stamp_bits);
        e
    }
}

pub fn pseudonym_of(ed_hex: &str, pq_hex: &str) -> String {
    let mut h = Sha256::new();
    h.update(hex::decode(ed_hex).unwrap_or_default());
    h.update(hex::decode(pq_hex).unwrap_or_default());
    hex::encode(&h.finalize()[..20])
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

/// Proof of Trust, step 1: signed by the SELLER, accepting `buyer` for a listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DealAccept {
    pub listing_id: String,
    pub buyer: String,
    pub created: u64,
}

/// Proof of Trust lifecycle steps (all value transfer happens off-network).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// Buyer states they settled off-network.
    BuyerPaid,
    /// Seller states the item was handed over / shipped.
    SellerShipped,
    /// Buyer confirms receipt.
    BuyerConfirmed,
    /// Buyer disputes.
    BuyerDisputed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DealStep {
    pub accept_id: String,
    pub step: Step,
    pub created: u64,
}

/// Buyer claims non-delivery for an accepted deal. Needs a stronger stamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub accept_id: String,
    pub reason: String,
    pub created: u64,
}

/// Extra stamp bits required for a Claim on top of the node minimum.
pub const CLAIM_EXTRA_BITS: u32 = 4;

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
    DealAccept(DealAccept),
    DealStep(DealStep),
    Claim(Claim),
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
            Payload::Claim(c) => if c.reason.len() > 500 { return bad("reason too long"); },
            Payload::DealReceipt(_) | Payload::DealAccept(_) | Payload::DealStep(_) => {}
        }
        Ok(())
    }
}

/// Signed envelope (v2). Both signatures cover (author, pq_pub, body). The
/// anti-spam stamp `nonce` sits OUTSIDE the signatures but is bound to them,
/// so a light client can sign and any node can add the stamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub author: String,
    pub pq_pub: String,
    pub body: String,
    pub sig: String,
    pub pq_sig: String,
    pub nonce: u64,
}

fn signing_bytes(author: &str, pq_pub: &str, body: &str) -> Vec<u8> {
    format!("lastchange-v2|{author}|{pq_pub}|{body}").into_bytes()
}

impl Envelope {
    fn core_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(signing_bytes(&self.author, &self.pq_pub, &self.body));
        h.update(self.sig.as_bytes());
        h.update(self.pq_sig.as_bytes());
        h.finalize().into()
    }
    fn stamp_seed(&self) -> [u8; 32] {
        let mut v = b"lastchange-stamp-v2|".to_vec();
        v.extend_from_slice(&self.core_hash());
        circuit_pow::seed_from(&v)
    }
    fn required_bits(&self, min: u32) -> u32 {
        if matches!(self.payload(), Ok(Payload::Claim(_))) { min + CLAIM_EXTRA_BITS } else { min }
    }
    /// Compute the anti-spam stamp (node side). Claims get CLAIM_EXTRA_BITS more.
    pub fn stamp(&mut self, bits: u32) {
        self.nonce = circuit_pow::solve(self.stamp_seed(), self.required_bits(bits));
    }
    pub fn stamp_bits(&self) -> u32 { circuit_pow::work_bits(self.stamp_seed(), self.nonce) }
    /// Content id: independent of the stamp nonce.
    pub fn id(&self) -> String { hex::encode(self.core_hash()) }
    pub fn author_pseudonym(&self) -> String { pseudonym_of(&self.author, &self.pq_pub) }
    pub fn payload(&self) -> Result<Payload, CoreError> { Ok(serde_json::from_str(&self.body)?) }

    /// Verify both signatures (hybrid: BOTH must be valid) and payload rules.
    pub fn verify_signatures(&self) -> Result<Payload, CoreError> {
        let msg = signing_bytes(&self.author, &self.pq_pub, &self.body);
        let pk: [u8; 32] = hex::decode(&self.author).ok().and_then(|v| v.try_into().ok()).ok_or(CoreError::BadKey)?;
        let vk = VerifyingKey::from_bytes(&pk).map_err(|_| CoreError::BadKey)?;
        let sig: [u8; 64] = hex::decode(&self.sig).ok().and_then(|v| v.try_into().ok()).ok_or(CoreError::BadSignature)?;
        vk.verify(&msg, &Signature::from_bytes(&sig)).map_err(|_| CoreError::BadSignature)?;
        let pqb = hex::decode(&self.pq_pub).map_err(|_| CoreError::BadKey)?;
        let enc_vk = ml_dsa::EncodedVerifyingKey::<MlDsa65>::try_from(pqb.as_slice()).map_err(|_| CoreError::BadKey)?;
        let pvk = ml_dsa::VerifyingKey::<MlDsa65>::decode(&enc_vk);
        let sb = hex::decode(&self.pq_sig).map_err(|_| CoreError::BadPqSignature)?;
        let enc_sig = ml_dsa::EncodedSignature::<MlDsa65>::try_from(sb.as_slice()).map_err(|_| CoreError::BadPqSignature)?;
        let psig = ml_dsa::Signature::<MlDsa65>::decode(&enc_sig).ok_or(CoreError::BadPqSignature)?;
        if !pvk.verify_with_context(&msg, PQ_CTX, &psig) { return Err(CoreError::BadPqSignature); }
        let p = self.payload()?;
        p.validate()?;
        Ok(p)
    }
    /// Full verification: signatures + stamp.
    pub fn verify(&self, min_stamp_bits: u32) -> Result<Payload, CoreError> {
        let p = self.verify_signatures()?;
        if self.stamp_bits() < self.required_bits(min_stamp_bits) { return Err(CoreError::WeakStamp); }
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
        assert_eq!(e.pq_sig.len() / 2, 3309); // ML-DSA-65 signature bytes
        assert_eq!(id.pseudonym().len(), 40);
    }
    #[test]
    fn tamper_detected() {
        let mut e = Identity::generate().seal(&listing(), 4);
        e.body = e.body.replace("bicycle", "car");
        assert!(matches!(e.verify(0), Err(CoreError::BadSignature)));
    }
    #[test]
    fn hybrid_both_required() {
        let a = Identity::generate(); let b = Identity::generate();
        let mut e = a.seal(&listing(), 0);
        let other = b.seal(&listing(), 0);
        // swap in a valid ed25519 sig from another key -> rejected
        let mut e2 = e.clone(); e2.sig = other.sig.clone();
        assert!(matches!(e2.verify_signatures(), Err(CoreError::BadSignature)));
        // corrupt ML-DSA sig only -> rejected even though ed25519 is valid
        let mut s = hex::decode(&e.pq_sig).unwrap(); s[100] ^= 1; e.pq_sig = hex::encode(s);
        assert!(e.verify_signatures().is_err());
        // swap pq key -> rejected
        let mut e3 = a.seal(&listing(), 0); e3.pq_pub = b.pq_public_hex();
        assert!(e3.verify_signatures().is_err());
    }
    #[test]
    fn client_signs_node_stamps() {
        let client = Identity::generate();
        let mut e = client.sign(&listing());
        let id_before = e.id();
        assert!(e.verify_signatures().is_ok());
        e.stamp(8); // done by a different party (the node)
        assert_eq!(e.id(), id_before);
        assert!(e.verify(8).is_ok());
    }
    #[test]
    fn weak_stamp_rejected() {
        let e = Identity::generate().seal(&listing(), 0);
        // Very unlikely that nonce 0 already satisfies 24 bits.
        if e.stamp_bits() < 24 { assert!(matches!(e.verify(24), Err(CoreError::WeakStamp))); }
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
    fn claim_needs_stronger_stamp() {
        let id = Identity::generate();
        let c = Payload::Claim(Claim { accept_id: "a".into(), reason: "never arrived".into(), created: 0 });
        let e = id.seal(&c, 6);
        assert!(e.verify(6).is_ok());
        assert!(e.stamp_bits() >= 10);
    }
    #[test]
    fn key_roundtrip() {
        let a = Identity::generate();
        let b = Identity::from_master(&a.master_bytes());
        assert_eq!(a.public_hex(), b.public_hex());
        assert_eq!(a.pq_public_hex(), b.pq_public_hex());
    }
}
