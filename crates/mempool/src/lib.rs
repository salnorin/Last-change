//! Mempool for signed chain entries (Bitcoin-Core-style admission policy,
//! but entries are marketplace envelopes, never coins).
//!
//! Admission = full validation (hybrid signatures, payload rules, anti-spam
//! stamp) + policy (dedupe, size cap, per-author quota, eviction of the
//! weakest stamp). Block templates take the strongest stamps first.
//!
//! `SettlementRecord` is an ON-CHAIN *record* of a settlement that happened
//! outside Last-change (e.g. cash, bank, an external chain). Both deal
//! parties co-sign amount, currency and an optional external tx hash. The
//! chain moves no value and keeps no account state.

use lc_core::{verify_bytes, CoreError, Envelope, Payload, Step};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const SETTLEMENT_DOMAIN: &str = "lastchange-settlement-v1";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PoolError {
    #[error("invalid entry: {0}")]
    Invalid(String),
    #[error("already known")]
    Duplicate,
    #[error("author quota exceeded")]
    AuthorQuota,
    #[error("pool full and entry stamp too weak")]
    PoolFull,
    #[error("entry too large")]
    TooLarge,
    #[error("conflicting settlement for this deal")]
    Conflict,
}

impl From<CoreError> for PoolError {
    fn from(e: CoreError) -> Self {
        PoolError::Invalid(e.to_string())
    }
}

/// Party keys: (ed25519 hex, ml-dsa hex) + their two signatures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartySig {
    pub ed: String,
    pub pq: String,
    pub sig: String,
    pub pq_sig: String,
}

/// The co-signed terms. Amount is a decimal string for display only; the
/// chain never interprets or sums it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementTerms {
    pub accept_id: String,
    pub amount: String,
    pub currency: String,
    #[serde(default)]
    pub external_tx: Option<String>,
    pub created: u64,
}

impl SettlementTerms {
    pub fn signing_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize terms")
    }
    pub fn validate(&self) -> Result<(), PoolError> {
        let bad = |s: &str| Err(PoolError::Invalid(s.into()));
        if self.accept_id.is_empty() || self.accept_id.len() > 128 {
            return bad("accept_id");
        }
        let a = self.amount.trim();
        if a.is_empty()
            || a.len() > 32
            || !a.chars().all(|c| c.is_ascii_digit() || c == '.')
            || a.matches('.').count() > 1
        {
            return bad("amount must be a plain decimal");
        }
        let c = &self.currency;
        if c.len() < 2 || c.len() > 12 || !c.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return bad("currency code");
        }
        if let Some(x) = &self.external_tx {
            if x.is_empty() || x.len() > 128 || !x.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                return bad("external_tx");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub terms: SettlementTerms,
    pub seller: PartySig,
    pub buyer: PartySig,
}

impl SettlementRecord {
    pub fn id(&self) -> String {
        let mut h = Sha256::new();
        h.update(SETTLEMENT_DOMAIN.as_bytes());
        h.update(self.terms.signing_bytes());
        h.update(self.seller.ed.as_bytes());
        h.update(self.buyer.ed.as_bytes());
        hex::encode(h.finalize())
    }
    /// Rules: terms well-formed, two DISTINCT parties, both hybrid sigs valid.
    pub fn verify(&self) -> Result<(), PoolError> {
        self.terms.validate()?;
        if self.seller.ed == self.buyer.ed || self.seller.pq == self.buyer.pq {
            return Err(PoolError::Invalid("parties must differ".into()));
        }
        let m = self.terms.signing_bytes();
        for p in [&self.seller, &self.buyer] {
            verify_bytes(SETTLEMENT_DOMAIN, &m, &p.ed, &p.pq, &p.sig, &p.pq_sig)?;
        }
        Ok(())
    }
}

/// Deal state machine rule: a deal may only be marked complete
/// (`BuyerConfirmed`) once a verified settlement record exists for it.
pub fn step_allowed(step: Step, accept_id: &str, settled: &HashSet<String>) -> bool {
    match step {
        Step::BuyerConfirmed => settled.contains(accept_id),
        _ => true,
    }
}

#[derive(Debug, Clone)]
pub enum PoolItem {
    Entry(Envelope),
    Settlement(SettlementRecord),
}

impl PoolItem {
    pub fn id(&self) -> String {
        match self {
            PoolItem::Entry(e) => e.id(),
            PoolItem::Settlement(s) => s.id(),
        }
    }
    fn author(&self) -> String {
        match self {
            PoolItem::Entry(e) => e.author.clone(),
            PoolItem::Settlement(s) => s.buyer.ed.clone(),
        }
    }
    fn size(&self) -> usize {
        match self {
            PoolItem::Entry(e) => serde_json::to_vec(e).map(|v| v.len()).unwrap_or(usize::MAX),
            PoolItem::Settlement(s) => serde_json::to_vec(s).map(|v| v.len()).unwrap_or(usize::MAX),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub max_items: usize,
    pub max_per_author: usize,
    pub max_item_bytes: usize,
    pub min_stamp_bits: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_items: 5000,
            max_per_author: 50,
            max_item_bytes: 64 * 1024,
            min_stamp_bits: 8,
        }
    }
}

struct Slot {
    item: PoolItem,
    priority: u32,
    seq: u64,
}

pub struct Mempool {
    cfg: Config,
    slots: HashMap<String, Slot>,
    per_author: HashMap<String, usize>,
    settled_deals: HashMap<String, String>,
    seq: u64,
}

impl Mempool {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            slots: HashMap::new(),
            per_author: HashMap::new(),
            settled_deals: HashMap::new(),
            seq: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
    pub fn contains(&self, id: &str) -> bool {
        self.slots.contains_key(id)
    }

    /// Validate and admit. `known` = ids already in the chain (reject replays).
    /// `settled` = deals with a settlement already on chain.
    pub fn insert(
        &mut self,
        item: PoolItem,
        known: &dyn Fn(&str) -> bool,
        settled: &HashSet<String>,
    ) -> Result<String, PoolError> {
        if item.size() > self.cfg.max_item_bytes {
            return Err(PoolError::TooLarge);
        }
        let id = item.id();
        if self.slots.contains_key(&id) || known(&id) {
            return Err(PoolError::Duplicate);
        }
        let priority = match &item {
            PoolItem::Entry(e) => {
                let p = e.verify(self.cfg.min_stamp_bits)?;
                if let Payload::DealStep(s) = &p {
                    let mut all = settled.clone();
                    all.extend(self.settled_deals.keys().cloned());
                    if !step_allowed(s.step, &s.accept_id, &all) {
                        return Err(PoolError::Invalid("deal not settled yet".into()));
                    }
                }
                e.stamp_bits()
            }
            PoolItem::Settlement(s) => {
                s.verify()?;
                let a = &s.terms.accept_id;
                if settled.contains(a) || self.settled_deals.contains_key(a) {
                    return Err(PoolError::Conflict);
                }
                u32::MAX // records of completed deals are never evicted for spam
            }
        };
        let author = item.author();
        if *self.per_author.get(&author).unwrap_or(&0) >= self.cfg.max_per_author {
            return Err(PoolError::AuthorQuota);
        }
        if self.slots.len() >= self.cfg.max_items {
            let weakest = self
                .slots
                .iter()
                .min_by_key(|(_, s)| (s.priority, std::cmp::Reverse(s.seq)))
                .map(|(k, s)| (k.clone(), s.priority));
            match weakest {
                Some((k, p)) if p < priority => {
                    self.remove(&k);
                }
                _ => return Err(PoolError::PoolFull),
            }
        }
        if let PoolItem::Settlement(s) = &item {
            self.settled_deals
                .insert(s.terms.accept_id.clone(), id.clone());
        }
        *self.per_author.entry(author).or_insert(0) += 1;
        self.seq += 1;
        self.slots.insert(
            id.clone(),
            Slot {
                item,
                priority,
                seq: self.seq,
            },
        );
        Ok(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<PoolItem> {
        let s = self.slots.remove(id)?;
        let a = s.item.author();
        if let Some(n) = self.per_author.get_mut(&a) {
            *n -= 1;
            if *n == 0 {
                self.per_author.remove(&a);
            }
        }
        if let PoolItem::Settlement(r) = &s.item {
            self.settled_deals.remove(&r.terms.accept_id);
        }
        Some(s.item)
    }

    /// Block template: highest priority first, then arrival order. Settlements
    /// sort before the deal steps that depend on them.
    pub fn template(&self, max: usize) -> Vec<PoolItem> {
        let mut v: BTreeMap<(std::cmp::Reverse<u32>, u64), &PoolItem> = BTreeMap::new();
        for s in self.slots.values() {
            v.insert((std::cmp::Reverse(s.priority), s.seq), &s.item);
        }
        v.into_values().take(max).cloned().collect()
    }

    /// Drop items that were included in a connected block.
    pub fn remove_confirmed(&mut self, ids: &[String]) {
        for id in ids {
            self.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_core::{DealStep, Identity, Listing, ListingKind};

    fn listing(t: &str) -> Payload {
        Payload::Listing(Listing {
            kind: ListingKind::Offer,
            title: t.into(),
            desc: "ok".into(),
            price_hint: "negotiable".into(),
            category: "misc".into(),
            expiry: 0,
            image_hashes: vec![],
            created: 0,
        })
    }
    fn none(_: &str) -> bool {
        false
    }
    fn cfg() -> Config {
        Config {
            min_stamp_bits: 2,
            ..Config::default()
        }
    }
    fn party(id: &Identity, t: &SettlementTerms) -> PartySig {
        let (sig, pq_sig) = id.sign_bytes(SETTLEMENT_DOMAIN, &t.signing_bytes());
        PartySig {
            ed: id.public_hex(),
            pq: id.pq_public_hex(),
            sig,
            pq_sig,
        }
    }
    fn record(deal: &str) -> SettlementRecord {
        let (s, b) = (Identity::generate(), Identity::generate());
        let terms = SettlementTerms {
            accept_id: deal.into(),
            amount: "25.50".into(),
            currency: "EUR".into(),
            external_tx: Some("ab12cd".into()),
            created: 1,
        };
        SettlementRecord {
            seller: party(&s, &terms),
            buyer: party(&b, &terms),
            terms,
        }
    }

    #[test]
    fn admits_valid_rejects_dup_and_replay() {
        let mut p = Mempool::new(cfg());
        let e = Identity::generate().seal(&listing("bike"), 2);
        let id = p
            .insert(PoolItem::Entry(e.clone()), &none, &HashSet::new())
            .unwrap();
        assert_eq!(
            p.insert(PoolItem::Entry(e.clone()), &none, &HashSet::new()),
            Err(PoolError::Duplicate)
        );
        let mut q = Mempool::new(cfg());
        let known = |x: &str| x == id;
        assert_eq!(
            q.insert(PoolItem::Entry(e), &known, &HashSet::new()),
            Err(PoolError::Duplicate)
        );
    }

    #[test]
    fn rejects_tampered_and_weak() {
        let mut p = Mempool::new(cfg());
        let mut e = Identity::generate().seal(&listing("bike"), 2);
        e.body = e.body.replace("bike", "bikes");
        assert!(matches!(
            p.insert(PoolItem::Entry(e), &none, &HashSet::new()),
            Err(PoolError::Invalid(_))
        ));
        let strict = Config {
            min_stamp_bits: 20,
            ..Config::default()
        };
        let w = Identity::generate().seal(&listing("x"), 0);
        if w.stamp_bits() < 20 {
            let mut p = Mempool::new(strict);
            assert!(p
                .insert(PoolItem::Entry(w), &none, &HashSet::new())
                .is_err());
        }
    }

    #[test]
    fn author_quota_and_eviction() {
        let id = Identity::generate();
        let mut p = Mempool::new(Config {
            max_per_author: 2,
            ..cfg()
        });
        for t in ["a", "b"] {
            p.insert(
                PoolItem::Entry(id.seal(&listing(t), 2)),
                &none,
                &HashSet::new(),
            )
            .unwrap();
        }
        assert_eq!(
            p.insert(
                PoolItem::Entry(id.seal(&listing("c"), 2)),
                &none,
                &HashSet::new()
            ),
            Err(PoolError::AuthorQuota)
        );
        let mut small = Mempool::new(Config {
            max_items: 1,
            ..cfg()
        });
        small
            .insert(
                PoolItem::Entry(Identity::generate().seal(&listing("w"), 2)),
                &none,
                &HashSet::new(),
            )
            .unwrap();
        let strong = Identity::generate().seal(&listing("s"), 12);
        small
            .insert(PoolItem::Entry(strong.clone()), &none, &HashSet::new())
            .unwrap();
        assert_eq!(small.len(), 1);
        assert!(small.contains(&strong.id()));
    }

    #[test]
    fn settlement_cosigned_and_gates_completion() {
        let mut p = Mempool::new(cfg());
        let buyer = Identity::generate();
        let done = Payload::DealStep(DealStep {
            accept_id: "deal1".into(),
            step: Step::BuyerConfirmed,
            created: 2,
        });
        let e = buyer.seal(&done, 2);
        assert!(p
            .insert(PoolItem::Entry(e.clone()), &none, &HashSet::new())
            .is_err());
        p.insert(
            PoolItem::Settlement(record("deal1")),
            &none,
            &HashSet::new(),
        )
        .unwrap();
        p.insert(PoolItem::Entry(e), &none, &HashSet::new())
            .unwrap();
        assert!(matches!(p.template(10)[0], PoolItem::Settlement(_)));
        assert_eq!(
            p.insert(
                PoolItem::Settlement(record("deal1")),
                &none,
                &HashSet::new()
            ),
            Err(PoolError::Conflict)
        );
    }

    #[test]
    fn settlement_rejects_bad_sig_same_party_bad_terms() {
        let mut r = record("d");
        r.terms.amount = "99".into();
        assert!(r.verify().is_err());
        let mut r2 = record("d");
        r2.buyer = r2.seller.clone();
        assert!(r2.verify().is_err());
        let mut t = record("d").terms;
        t.amount = "-1".into();
        assert!(t.validate().is_err());
        t.amount = "1".into();
        t.currency = "e u r".into();
        assert!(t.validate().is_err());
        assert!(record("d").verify().is_ok());
    }

    #[test]
    fn confirmed_removed() {
        let mut p = Mempool::new(cfg());
        let id = p
            .insert(
                PoolItem::Entry(Identity::generate().seal(&listing("z"), 2)),
                &none,
                &HashSet::new(),
            )
            .unwrap();
        p.remove_confirmed(&[id]);
        assert!(p.is_empty());
    }
}
