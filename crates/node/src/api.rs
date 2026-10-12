//! Local HTTP/JSON + WebSocket API (bind to 127.0.0.1 by default).
use crate::{items, now};
use axum::{extract::{Path, Query, State, ws::{Message, WebSocket, WebSocketUpgrade}}, http::StatusCode,
    response::{Html, IntoResponse, Response}, routing::{get, post}, Json, Router};
use lc_core::*;
use lc_ledger::Ledger;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<Mutex<Ledger>>,
    pub id: Arc<Identity>,
    pub stamp_bits: u32,
    pub net: mpsc::Sender<lc_net::Command>,
    pub events: broadcast::Sender<String>,
}

type R = Result<Json<Value>, (StatusCode, String)>;
fn err<E: std::fmt::Display>(e: E) -> (StatusCode, String) { (StatusCode::BAD_REQUEST, e.to_string()) }

impl AppState {
    /// Verify, store, broadcast to peers and websocket clients.
    pub async fn ingest(&self, env: Envelope) -> Result<bool, String> {
        let new = self.ledger.lock().unwrap().append(&env).map_err(|e| e.to_string())?;
        if new {
            let _ = self.net.send(lc_net::Command::Publish(env.clone())).await;
            let _ = self.events.send(json!({"id": env.id(), "author": env.author, "body": env.body}).to_string());
        }
        Ok(new)
    }
    async fn seal(&self, p: Payload) -> R {
        let (id, bits) = (self.id.clone(), self.stamp_bits);
        let env = tokio::task::spawn_blocking(move || id.seal(&p, bits)).await.map_err(err)?;
        self.ingest(env.clone()).await.map_err(err)?;
        Ok(Json(json!({"id": env.id()})))
    }
}

pub fn router(st: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { Html(include_str!("../../../web/index.html")) }))
        .route("/api/whoami", get(whoami))
        .route("/api/listings", get(listings))
        .route("/api/entries", get(entries))
        .route("/api/trust/:pk", get(trust))
        .route("/api/post", post(post_listing))
        .route("/api/accept", post(accept))
        .route("/api/step", post(step))
        .route("/api/claim", post(claim))
        .route("/api/complete", post(complete))
        .route("/api/rate", post(rate))
        .route("/api/report", post(report))
        .route("/api/envelope", post(envelope))
        .route("/api/stamp", post(stamp))
        .route("/ws", get(ws))
        .with_state(st)
}

async fn whoami(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"pseudonym": s.id.pseudonym(), "pubkey": s.id.public_hex(), "ml_dsa_65": s.id.pq_public_hex(),
        "kem": s.id.kem_public(), "stamp_bits": s.stamp_bits}))
}

#[derive(Deserialize)]
struct LQ { q: Option<String>, kind: Option<String>, category: Option<String> }

async fn listings(State(s): State<AppState>, Query(q): Query<LQ>) -> R {
    let led = s.ledger.lock().unwrap();
    let it = items(&led).map_err(err)?;
    let rep = lc_reputation::compute(&it, now(), &Default::default());
    let needle = q.q.unwrap_or_default().to_lowercase();
    let mut out = vec![];
    for i in &it {
        let Payload::Listing(l) = &i.payload else { continue };
        let r = rep.get(&i.author);
        if r.is_some_and(|r| r.struck) || l.expiry < now() || led.is_blocked(&i.author).unwrap_or(false)
            || led.is_blocked(&i.pseudonym.clone()).unwrap_or(false) || led.is_blocked(&i.id).unwrap_or(false) { continue; }
        if q.kind.as_deref().is_some_and(|k| !format!("{:?}", l.kind).eq_ignore_ascii_case(k)) { continue; }
        if q.category.as_deref().is_some_and(|c| !l.category.eq_ignore_ascii_case(c)) { continue; }
        if !format!("{} {} {}", l.title, l.desc, l.category).to_lowercase().contains(&needle) { continue; }
        out.push(json!({"id": i.id, "seller": i.author, "seller_pseudonym": i.pseudonym.clone(), "listing": l,
            "rep": r.map(|r| json!({"mean": r.mean, "count": r.count, "reports": r.reports, "strikes": r.strikes}))}));
    }
    Ok(Json(Value::Array(out)))
}

async fn entries(State(s): State<AppState>) -> R {
    let led = s.ledger.lock().unwrap();
    let v: Vec<Value> = led.all().map_err(err)?.into_iter().map(|e| json!({"id": e.id, "author": e.env.author, "payload": e.payload})).collect();
    Ok(Json(json!({"merkle_root": led.merkle_root().map_err(err)?, "entries": v})))
}

async fn trust(State(s): State<AppState>, Path(pk): Path<String>) -> R {
    let it = items(&s.ledger.lock().unwrap()).map_err(err)?;
    let r = lc_reputation::compute(&it, now(), &Default::default());
    Ok(Json(match r.get(&pk) { Some(x) => json!({"mean": x.mean, "weight": x.weight, "count": x.count, "reports": x.reports,
        "strikes": x.strikes, "false_claims": x.false_claims, "struck": x.struck}), None => json!(null) }))
}

#[derive(Deserialize)]
struct PostReq { kind: String, title: String, #[serde(default)] desc: String, #[serde(default)] price_hint: String,
    #[serde(default)] category: String, #[serde(default)] days: Option<u64>, #[serde(default)] images: Vec<String> }
async fn post_listing(State(s): State<AppState>, Json(r): Json<PostReq>) -> R {
    let kind = if r.kind == "request" { ListingKind::Request } else { ListingKind::Offer };
    s.seal(Payload::Listing(Listing { kind, title: r.title, desc: r.desc, price_hint: r.price_hint,
        category: if r.category.is_empty() { "misc".into() } else { r.category }, expiry: now() + r.days.unwrap_or(30) * 86400,
        image_hashes: r.images, created: now() })).await
}

fn get_entry(s: &AppState, id: &str) -> Result<Envelope, (StatusCode, String)> {
    s.ledger.lock().unwrap().get(id).map_err(err)?.ok_or_else(|| err("unknown id"))
}

#[derive(Deserialize)] struct BuyerReq { listing_id: String, buyer: String }
async fn accept(State(s): State<AppState>, Json(r): Json<BuyerReq>) -> R {
    if get_entry(&s, &r.listing_id)?.author != s.id.public_hex() { return Err(err("not your listing")); }
    s.seal(Payload::DealAccept(DealAccept { listing_id: r.listing_id, buyer: r.buyer, created: now() })).await
}
async fn complete(State(s): State<AppState>, Json(r): Json<BuyerReq>) -> R {
    if get_entry(&s, &r.listing_id)?.author != s.id.public_hex() { return Err(err("not your listing")); }
    s.seal(Payload::DealReceipt(DealReceipt { listing_id: r.listing_id, buyer: r.buyer })).await
}
#[derive(Deserialize)] struct StepReq { accept_id: String, step: Step }
async fn step(State(s): State<AppState>, Json(r): Json<StepReq>) -> R {
    get_entry(&s, &r.accept_id)?;
    s.seal(Payload::DealStep(DealStep { accept_id: r.accept_id, step: r.step, created: now() })).await
}
#[derive(Deserialize)] struct ClaimReq { accept_id: String, #[serde(default)] reason: String }
async fn claim(State(s): State<AppState>, Json(r): Json<ClaimReq>) -> R {
    get_entry(&s, &r.accept_id)?;
    s.seal(Payload::Claim(Claim { accept_id: r.accept_id, reason: r.reason, created: now() })).await
}
#[derive(Deserialize)] struct RateReq { receipt_id: String, score: u8, #[serde(default)] comment: String }
async fn rate(State(s): State<AppState>, Json(r): Json<RateReq>) -> R {
    let env = get_entry(&s, &r.receipt_id)?;
    let Ok(Payload::DealReceipt(d)) = env.payload() else { return Err(err("not a receipt")) };
    if d.buyer != s.id.public_hex() { return Err(err("receipt names another buyer")); }
    s.seal(Payload::Rating(Rating { listing_id: d.listing_id, seller: env.author, receipt_id: r.receipt_id, score: r.score, comment: r.comment })).await
}
#[derive(Deserialize)] struct ReportReq { target_id: String, reason: String }
async fn report(State(s): State<AppState>, Json(r): Json<ReportReq>) -> R {
    s.seal(Payload::Report(Report { target_id: r.target_id, reason: r.reason })).await
}
/// Pre-signed envelope from an external client (e.g. browser key). Fully verified.
async fn envelope(State(s): State<AppState>, Json(env): Json<Envelope>) -> R {
    let new = s.ingest(env.clone()).await.map_err(err)?;
    Ok(Json(json!({"id": env.id(), "new": new})))
}

/// Light client flow: the client signs (both algorithms), this node computes the
/// anti-spam stamp, verifies everything and publishes. Clients never do PoW.
async fn stamp(State(s): State<AppState>, Json(mut env): Json<Envelope>) -> R {
    env.verify_signatures().map_err(err)?;
    let bits = s.stamp_bits;
    let env = tokio::task::spawn_blocking(move || { env.stamp(bits); env }).await.map_err(err)?;
    let new = s.ingest(env.clone()).await.map_err(err)?;
    Ok(Json(json!({"id": env.id(), "nonce": env.nonce, "new": new})))
}

async fn ws(State(s): State<AppState>, up: WebSocketUpgrade) -> Response {
    up.on_upgrade(move |sock| ws_loop(sock, s.events.subscribe())).into_response()
}
async fn ws_loop(mut sock: WebSocket, mut rx: broadcast::Receiver<String>) {
    while let Ok(m) = rx.recv().await {
        if sock.send(Message::Text(m)).await.is_err() { break; }
    }
}
