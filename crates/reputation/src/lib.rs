//! Reputation from ratings. A rating only counts if:
//!  * it references a DealReceipt signed by the seller naming the rater as buyer,
//!  * the receipt is for a listing authored by that seller,
//!  * rater != seller and only one rating per receipt.
//! Weight = rater's own reputation (bounded), with exponential time decay.

use lc_core::{Payload};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub author: String,
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
}

pub struct Params {
    pub half_life_secs: f64,
    pub base_weight: f64,
    pub iterations: usize,
}
impl Default for Params {
    fn default() -> Self { Self { half_life_secs: 180.0 * 86400.0, base_weight: 0.2, iterations: 3 } }
}

struct Valid { rater: String, seller: String, score: f64, time: u64 }

/// Filter ratings to those backed by a valid seller-signed receipt.
fn valid_ratings(items: &[Item]) -> Vec<Valid> {
    let mut listing_author = HashMap::new();
    let mut receipts = HashMap::new();
    for it in items {
        match &it.payload {
            Payload::Listing(_) => { listing_author.insert(it.id.clone(), it.author.clone()); }
            Payload::DealReceipt(r) => { receipts.insert(it.id.clone(), (it.author.clone(), r.clone())); }
            _ => {}
        }
    }
    let mut used = HashSet::new();
    let mut out = vec![];
    for it in items {
        let Payload::Rating(r) = &it.payload else { continue };
        let Some((rcpt_author, rcpt)) = receipts.get(&r.receipt_id) else { continue };
        let ok = rcpt_author == &r.seller
            && rcpt.buyer == it.author
            && rcpt.listing_id == r.listing_id
            && listing_author.get(&r.listing_id) == Some(&r.seller)
            && it.author != r.seller
            && used.insert(r.receipt_id.clone());
        if ok { out.push(Valid { rater: it.author.clone(), seller: r.seller.clone(), score: r.score as f64, time: it.time }); }
    }
    out
}

pub fn compute(items: &[Item], now: u64, p: &Params) -> HashMap<String, Score> {
    let valid = valid_ratings(items);
    let mut reports: HashMap<String, usize> = HashMap::new();
    let by_id: HashMap<&str, &str> = items.iter().map(|i| (i.id.as_str(), i.author.as_str())).collect();
    for it in items {
        if let Payload::Report(r) = &it.payload {
            let target = by_id.get(r.target_id.as_str()).map(|s| s.to_string()).unwrap_or(r.target_id.clone());
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
            let rw = scores.get(&v.rater).map(|s| p.base_weight + (s.weight.min(10.0) / 10.0) * (s.mean / 5.0)).unwrap_or(p.base_weight);
            let w = rw * decay;
            let e = acc.entry(v.seller.clone()).or_default();
            e.0 += w * v.score; e.1 += w; e.2 += 1;
        }
        scores = acc.into_iter().map(|(k, (s, w, c))| (k.clone(), Score {
            mean: if w > 0.0 { s / w } else { 0.0 }, weight: w, count: c, reports: *reports.get(&k).unwrap_or(&0) })).collect();
    }
    for (k, r) in reports { scores.entry(k).or_insert(Score { mean: 0.0, weight: 0.0, count: 0, reports: r }); }
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_core::*;
    fn item(id: &str, author: &str, p: Payload, t: u64) -> Item { Item { id: id.into(), author: author.into(), payload: p, time: t } }
    fn listing() -> Payload { Payload::Listing(Listing { kind: ListingKind::Offer, title: "t".into(), desc: "".into(), price_hint: "".into(), category: "".into(), expiry: 0, image_hashes: vec![], created: 0 }) }
    fn rating(seller: &str, rcpt: &str, s: u8) -> Payload { Payload::Rating(Rating { listing_id: "L".into(), seller: seller.into(), receipt_id: rcpt.into(), score: s, comment: "".into() }) }
    fn rcpt(buyer: &str) -> Payload { Payload::DealReceipt(DealReceipt { listing_id: "L".into(), buyer: buyer.into() }) }

    #[test]
    fn rating_needs_receipt() {
        let items = vec![item("L", "S", listing(), 0), item("R1", "X", rating("S", "none", 1), 0)];
        assert!(compute(&items, 0, &Params::default()).get("S").is_none());
    }
    #[test]
    fn valid_rating_counts_once() {
        let items = vec![item("L", "S", listing(), 0), item("C", "S", rcpt("B"), 0),
            item("R1", "B", rating("S", "C", 5), 0), item("R2", "B", rating("S", "C", 1), 0)];
        let s = compute(&items, 0, &Params::default())["S"];
        assert_eq!(s.count, 1);
        assert!((s.mean - 5.0).abs() < 1e-9);
    }
    #[test]
    fn wrong_buyer_or_self_rating_rejected() {
        let items = vec![item("L", "S", listing(), 0), item("C", "S", rcpt("B"), 0),
            item("R1", "Z", rating("S", "C", 5), 0)];
        assert!(compute(&items, 0, &Params::default()).get("S").is_none());
        let items = vec![item("L", "S", listing(), 0), item("C", "S", rcpt("S"), 0), item("R", "S", rating("S", "C", 5), 0)];
        assert!(compute(&items, 0, &Params::default()).get("S").is_none());
    }
    #[test]
    fn decay_reduces_weight() {
        let items = vec![item("L", "S", listing(), 0), item("C", "S", rcpt("B"), 0), item("R", "B", rating("S", "C", 4), 0)];
        let p = Params::default();
        let fresh = compute(&items, 0, &p)["S"].weight;
        let old = compute(&items, (p.half_life_secs as u64) * 2, &p)["S"].weight;
        assert!((old - fresh / 4.0).abs() < 1e-9);
    }
    #[test]
    fn reputable_rater_weighs_more() {
        // B has reputation (rated by C as seller), A has none. Both rate S.
        let mut items = vec![item("L", "S", listing(), 0), item("C1", "S", rcpt("A"), 0), item("C2", "S", rcpt("B"), 0),
            item("R1", "A", rating("S", "C1", 1), 0), item("R2", "B", rating("S", "C2", 5), 0)];
        items.push(item("LB", "B", listing(), 0));
        items.push(item("CB", "B", Payload::DealReceipt(DealReceipt { listing_id: "LB".into(), buyer: "C".into() }), 0));
        items.push(item("RB", "C", Payload::Rating(Rating { listing_id: "LB".into(), seller: "B".into(), receipt_id: "CB".into(), score: 5, comment: "".into() }), 0));
        let s = compute(&items, 0, &Params::default())["S"];
        assert!(s.mean > 3.0, "mean {}", s.mean);
    }
}
