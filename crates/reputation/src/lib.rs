//! Reputation from ratings. A rating only counts if:
//!  * it references a DealReceipt signed by the seller naming the rater as buyer,
//!  * the receipt is for a listing authored by that seller,
//!  * rater != seller and only one rating per receipt.
//!
//! Weight = rater's own reputation (bounded), with exponential time decay.

use lc_core::Payload;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub author: String,
    /// Display pseudonym (hash of both public keys).
    pub pseudonym: String,
    pub payload: Payload,
    /// Unix seconds when seen (or listing.created).
    pub time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    /// Weighted mean 1..5 (0 if no valid ratings).
    pub mean: f64,
    /// Effective (weighted, decayed) number of ratings.
    pub weight: f64,
    pub count: usize,
    pub reports: usize,
    /// Proof of Trust strikes (valid non-delivery claims against this seller).
    pub strikes: usize,
    /// Invalid claims filed by this identity.
    pub false_claims: usize,
    /// Strike limit reached: listings should be hidden, reputation zeroed.
    pub struck: bool,
}

pub struct Params {
    pub half_life_secs: f64,
    pub base_weight: f64,
    pub iterations: usize,
    /// Seller must post SellerShipped within this time after acceptance.
    pub claim_timeout_secs: u64,
    /// Strikes before an identity is struck (default 1 = one-strike policy).
    pub strike_limit: usize,
}
impl Default for Params {
    fn default() -> Self {
        Self {
            half_life_secs: 180.0 * 86400.0,
            base_weight: 0.2,
            iterations: 3,
            claim_timeout_secs: 14 * 86400,
            strike_limit: 1,
        }
    }
}

/// Proof of Trust evaluation: returns (strikes per seller, false claims per claimer).
pub fn trust(items: &[Item], p: &Params) -> (HashMap<String, usize>, HashMap<String, usize>) {
    use lc_core::Step;
    let mut listing_author = HashMap::new();
    let mut accepts = HashMap::new();
    for it in items {
        match &it.payload {
            Payload::Listing(_) => {
                listing_author.insert(it.id.as_str(), it.author.as_str());
            }
            Payload::DealAccept(a) => {
                accepts.insert(it.id.as_str(), (it.author.as_str(), a));
            }
            _ => {}
        }
    }
    // valid accepts: seller authored the listing
    accepts.retain(|_, (seller, a)| {
        listing_author.get(a.listing_id.as_str()) == Some(seller) && a.buyer != *seller
    });
    let mut shipped: HashMap<&str, u64> = HashMap::new();
    let mut confirmed: HashSet<&str> = HashSet::new();
    for it in items {
        if let Payload::DealStep(s) = &it.payload {
            let Some((seller, a)) = accepts.get(s.accept_id.as_str()) else {
                continue;
            };
            match s.step {
                Step::SellerShipped if it.author == *seller => {
                    shipped.entry(s.accept_id.as_str()).or_insert(s.created);
                }
                Step::BuyerConfirmed if it.author == a.buyer => {
                    confirmed.insert(s.accept_id.as_str());
                }
                _ => {}
            }
        }
    }
    let mut strikes: HashMap<String, usize> = HashMap::new();
    let mut false_claims: HashMap<String, usize> = HashMap::new();
    let mut used = HashSet::new();
    for it in items {
        let Payload::Claim(c) = &it.payload else {
            continue;
        };
        let valid = match accepts.get(c.accept_id.as_str()) {
            Some((seller, a)) => {
                let deadline = a.created + p.claim_timeout_secs;
                let shipped_in_time = shipped
                    .get(c.accept_id.as_str())
                    .is_some_and(|t| *t <= deadline);
                if a.buyer == it.author
                    && c.created >= deadline
                    && !shipped_in_time
                    && !confirmed.contains(c.accept_id.as_str())
                {
                    if used.insert(c.accept_id.clone()) {
                        *strikes.entry(seller.to_string()).or_default() += 1;
                    }
                    true
                } else {
                    false
                }
            }
            None => false,
        };
        if !valid {
            *false_claims.entry(it.author.clone()).or_default() += 1;
        }
    }
    (strikes, false_claims)
}

struct Valid {
    rater: String,
    seller: String,
    score: f64,
    time: u64,
}

/// Filter ratings to those backed by a valid seller-signed receipt.
fn valid_ratings(items: &[Item]) -> Vec<Valid> {
    let mut listing_author = HashMap::new();
    let mut receipts = HashMap::new();
    for it in items {
        match &it.payload {
            Payload::Listing(_) => {
                listing_author.insert(it.id.clone(), it.author.clone());
            }
            Payload::DealReceipt(r) => {
                receipts.insert(it.id.clone(), (it.author.clone(), r.clone()));
            }
            _ => {}
        }
    }
    let mut used = HashSet::new();
    let mut out = vec![];
    for it in items {
        let Payload::Rating(r) = &it.payload else {
            continue;
        };
        let Some((rcpt_author, rcpt)) = receipts.get(&r.receipt_id) else {
            continue;
        };
        let ok = rcpt_author == &r.seller
            && rcpt.buyer == it.author
            && rcpt.listing_id == r.listing_id
            && listing_author.get(&r.listing_id) == Some(&r.seller)
            && it.author != r.seller
            && used.insert(r.receipt_id.clone());
        if ok {
            out.push(Valid {
                rater: it.author.clone(),
                seller: r.seller.clone(),
                score: r.score as f64,
                time: it.time,
            });
        }
    }
    out
}

pub fn compute(items: &[Item], now: u64, p: &Params) -> HashMap<String, Score> {
    let (strikes, false_claims) = trust(items, p);
    let valid = valid_ratings(items);
    let mut reports: HashMap<String, usize> = HashMap::new();
    let by_id: HashMap<&str, &str> = items
        .iter()
        .map(|i| (i.id.as_str(), i.author.as_str()))
        .collect();
    for it in items {
        if let Payload::Report(r) = &it.payload {
            let target = by_id
                .get(r.target_id.as_str())
                .map(|s| s.to_string())
                .unwrap_or(r.target_id.clone());
            *reports.entry(target).or_default() += 1;
        }
    }
    let mut scores: HashMap<String, Score> = HashMap::new();
    for _ in 0..p.iterations.max(1) {
        let mut acc: HashMap<String, (f64, f64, usize)> = HashMap::new();
        for v in &valid {
            let age = now.saturating_sub(v.time) as f64;
            let decay = 0.5f64.powf(age / p.half_life_secs);
            // rater weight: base + bounded function of their own reputation
            let fc = *false_claims.get(&v.rater).unwrap_or(&0) as i32;
            let rw = 0.5f64.powi(fc)
                * scores
                    .get(&v.rater)
                    .map(|s| p.base_weight + (s.weight.min(10.0) / 10.0) * (s.mean / 5.0))
                    .unwrap_or(p.base_weight);
            let w = rw * decay;
            let e = acc.entry(v.seller.clone()).or_default();
            e.0 += w * v.score;
            e.1 += w;
            e.2 += 1;
        }
        scores = acc
            .into_iter()
            .map(|(k, (s, w, c))| {
                (
                    k.clone(),
                    Score {
                        mean: if w > 0.0 { s / w } else { 0.0 },
                        weight: w,
                        count: c,
                        reports: *reports.get(&k).unwrap_or(&0),
                        strikes: 0,
                        false_claims: 0,
                        struck: false,
                    },
                )
            })
            .collect();
    }
    let empty = Score {
        mean: 0.0,
        weight: 0.0,
        count: 0,
        reports: 0,
        strikes: 0,
        false_claims: 0,
        struck: false,
    };
    for (k, r) in reports {
        scores.entry(k).or_insert(Score {
            reports: r,
            ..empty
        });
    }
    for k in strikes.keys().chain(false_claims.keys()) {
        scores.entry(k.clone()).or_insert(empty);
    }
    for (k, sc) in scores.iter_mut() {
        sc.strikes = *strikes.get(k).unwrap_or(&0);
        sc.false_claims = *false_claims.get(k).unwrap_or(&0);
        sc.weight *= 0.5f64.powi(sc.false_claims as i32);
        if sc.strikes >= p.strike_limit.max(1) {
            sc.struck = true;
            sc.mean = 0.0;
            sc.weight = 0.0;
        }
    }
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_core::*;
    fn item(id: &str, author: &str, p: Payload, t: u64) -> Item {
        Item {
            id: id.into(),
            author: author.into(),
            pseudonym: author.into(),
            payload: p,
            time: t,
        }
    }
    fn listing() -> Payload {
        Payload::Listing(Listing {
            kind: ListingKind::Offer,
            title: "t".into(),
            desc: "".into(),
            price_hint: "".into(),
            category: "".into(),
            expiry: 0,
            image_hashes: vec![],
            created: 0,
        })
    }
    fn rating(seller: &str, rcpt: &str, s: u8) -> Payload {
        Payload::Rating(Rating {
            listing_id: "L".into(),
            seller: seller.into(),
            receipt_id: rcpt.into(),
            score: s,
            comment: "".into(),
        })
    }
    fn rcpt(buyer: &str) -> Payload {
        Payload::DealReceipt(DealReceipt {
            listing_id: "L".into(),
            buyer: buyer.into(),
        })
    }

    #[test]
    fn rating_needs_receipt() {
        let items = vec![
            item("L", "S", listing(), 0),
            item("R1", "X", rating("S", "none", 1), 0),
        ];
        assert!(!compute(&items, 0, &Params::default()).contains_key("S"));
    }
    #[test]
    fn valid_rating_counts_once() {
        let items = vec![
            item("L", "S", listing(), 0),
            item("C", "S", rcpt("B"), 0),
            item("R1", "B", rating("S", "C", 5), 0),
            item("R2", "B", rating("S", "C", 1), 0),
        ];
        let s = compute(&items, 0, &Params::default())["S"];
        assert_eq!(s.count, 1);
        assert!((s.mean - 5.0).abs() < 1e-9);
    }
    #[test]
    fn wrong_buyer_or_self_rating_rejected() {
        let items = vec![
            item("L", "S", listing(), 0),
            item("C", "S", rcpt("B"), 0),
            item("R1", "Z", rating("S", "C", 5), 0),
        ];
        assert!(!compute(&items, 0, &Params::default()).contains_key("S"));
        let items = vec![
            item("L", "S", listing(), 0),
            item("C", "S", rcpt("S"), 0),
            item("R", "S", rating("S", "C", 5), 0),
        ];
        assert!(!compute(&items, 0, &Params::default()).contains_key("S"));
    }
    fn accept(buyer: &str, t: u64) -> Payload {
        Payload::DealAccept(DealAccept {
            listing_id: "L".into(),
            buyer: buyer.into(),
            created: t,
        })
    }
    fn step(st: Step, t: u64) -> Payload {
        Payload::DealStep(DealStep {
            accept_id: "A".into(),
            step: st,
            created: t,
        })
    }
    fn claim(t: u64) -> Payload {
        Payload::Claim(Claim {
            accept_id: "A".into(),
            reason: "".into(),
            created: t,
        })
    }

    #[test]
    fn non_delivery_claim_strikes_seller() {
        let p = Params::default();
        let late = p.claim_timeout_secs + 1;
        let items = vec![
            item("L", "S", listing(), 0),
            item("A", "S", accept("B", 0), 0),
            item("P", "B", step(Step::BuyerPaid, 1), 1),
            item("K", "B", claim(late), late),
        ];
        let s = compute(&items, late, &p)["S"];
        assert_eq!(s.strikes, 1);
        assert!(s.struck);
        assert_eq!(s.weight, 0.0);
    }
    #[test]
    fn early_or_false_claim_hurts_claimer() {
        let p = Params::default();
        let late = p.claim_timeout_secs + 1;
        // too early
        let items = vec![
            item("L", "S", listing(), 0),
            item("A", "S", accept("B", 0), 0),
            item("K", "B", claim(5), 5),
        ];
        let r = compute(&items, 5, &p);
        assert!(r.get("S").is_none_or(|s| !s.struck));
        assert_eq!(r["B"].false_claims, 1);
        // seller shipped in time
        let items = vec![
            item("L", "S", listing(), 0),
            item("A", "S", accept("B", 0), 0),
            item("H", "S", step(Step::SellerShipped, 10), 10),
            item("K", "B", claim(late), late),
        ];
        let r = compute(&items, late, &p);
        assert!(r.get("S").is_none_or(|s| !s.struck));
        assert_eq!(r["B"].false_claims, 1);
        // stranger claims
        let items = vec![
            item("L", "S", listing(), 0),
            item("A", "S", accept("B", 0), 0),
            item("K", "X", claim(late), late),
        ];
        assert_eq!(compute(&items, late, &p)["X"].false_claims, 1);
    }
    #[test]
    fn strike_limit_configurable() {
        let p = Params {
            strike_limit: 2,
            ..Default::default()
        };
        let late = p.claim_timeout_secs + 1;
        let items = vec![
            item("L", "S", listing(), 0),
            item("A", "S", accept("B", 0), 0),
            item("K", "B", claim(late), late),
        ];
        let s = compute(&items, late, &p)["S"];
        assert_eq!(s.strikes, 1);
        assert!(!s.struck);
    }
    #[test]
    fn decay_reduces_weight() {
        let items = vec![
            item("L", "S", listing(), 0),
            item("C", "S", rcpt("B"), 0),
            item("R", "B", rating("S", "C", 4), 0),
        ];
        let p = Params::default();
        let fresh = compute(&items, 0, &p)["S"].weight;
        let old = compute(&items, (p.half_life_secs as u64) * 2, &p)["S"].weight;
        assert!((old - fresh / 4.0).abs() < 1e-9);
    }
    #[test]
    fn reputable_rater_weighs_more() {
        // B has reputation (rated by C as seller), A has none. Both rate S.
        let mut items = vec![
            item("L", "S", listing(), 0),
            item("C1", "S", rcpt("A"), 0),
            item("C2", "S", rcpt("B"), 0),
            item("R1", "A", rating("S", "C1", 1), 0),
            item("R2", "B", rating("S", "C2", 5), 0),
        ];
        items.push(item("LB", "B", listing(), 0));
        items.push(item(
            "CB",
            "B",
            Payload::DealReceipt(DealReceipt {
                listing_id: "LB".into(),
                buyer: "C".into(),
            }),
            0,
        ));
        items.push(item(
            "RB",
            "C",
            Payload::Rating(Rating {
                listing_id: "LB".into(),
                seller: "B".into(),
                receipt_id: "CB".into(),
                score: 5,
                comment: "".into(),
            }),
            0,
        ));
        let s = compute(&items, 0, &Params::default())["S"];
        assert!(s.mean > 3.0, "mean {}", s.mean);
    }
}
