//! `lastchange` CLI. Data lives in a local directory (default ./.lastchange):
//! `key` (32-byte ed25519 secret, nothing else) and `ledger.db`.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use lc_core::*;
use lc_ledger::Ledger;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

mod api;

#[derive(Parser)]
#[command(name = "lastchange", about = "Pseudonymous P2P classifieds with reputation.")]
struct Cli {
    #[arg(long, default_value = ".lastchange", global = true)]
    dir: PathBuf,
    #[arg(long, default_value_t = DEFAULT_STAMP_BITS, global = true)]
    stamp_bits: u32,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a local keypair (your only identity).
    Keygen,
    /// Show your pseudonym and public key.
    Whoami,
    /// Post an offer or request.
    Post {
        #[arg(value_parser = ["offer", "request"])] kind: String,
        #[arg(long)] title: String,
        #[arg(long, default_value = "")] desc: String,
        #[arg(long, default_value = "")] price_hint: String,
        #[arg(long, default_value = "misc")] category: String,
        #[arg(long, default_value_t = 30)] days: u64,
        #[arg(long)] image: Vec<String>,
    },
    /// List active listings with seller reputation.
    List { #[arg(long)] kind: Option<String> },
    /// Search listings by text.
    Search { query: String },
    /// Seller: confirm a completed deal with a buyer (enables their rating).
    CompleteDeal { listing_id: String, #[arg(long)] buyer: String },
    /// Buyer: rate a seller after receiving, referencing the seller's receipt.
    Rate { receipt_id: String, #[arg(long)] score: u8, #[arg(long, default_value = "")] comment: String },
    /// Seller: accept a buyer for a listing (Proof of Trust step 1).
    Accept { listing_id: String, #[arg(long)] buyer: String },
    /// Record a lifecycle step: buyer-paid | seller-shipped | buyer-confirmed | buyer-disputed.
    Step { accept_id: String, #[arg(value_parser = ["buyer-paid", "seller-shipped", "buyer-confirmed", "buyer-disputed"])] step: String },
    /// Buyer: non-delivery claim after the timeout (costs a stronger stamp; false claims hurt you).
    Claim { accept_id: String, #[arg(long, default_value = "not delivered")] reason: String },
    /// Show trust status (score, strikes, false claims) of a pubkey.
    Trust { pubkey: String },
    /// Flag a listing or author for others (gossiped).
    Report { target_id: String, #[arg(long)] reason: String },
    /// Locally block a pseudonym, pubkey or entry id.
    Block { key: String },
    /// Show ledger stats and Merkle root.
    Status,
    /// Run P2P node + local HTTP/JSON/WebSocket API and web app.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")] http: String,
        #[arg(long, default_value = "/ip4/0.0.0.0/tcp/0")] listen: String,
        #[arg(long)] dial: Vec<String>,
    },
    /// Run a P2P node: sync local entries and receive peers' entries.
    Peers {
        #[arg(long, default_value = "/ip4/0.0.0.0/tcp/0")] listen: String,
        #[arg(long)] dial: Vec<String>,
    },
}

pub(crate) fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() }

fn load_id(dir: &std::path::Path) -> Result<Identity> {
    let b = std::fs::read(dir.join("key")).context("no key; run `lastchange keygen`")?;
    let a: [u8; 32] = b.try_into().map_err(|_| anyhow::anyhow!("bad key file"))?;
    Ok(Identity::from_secret_bytes(&a))
}

pub(crate) fn items(led: &Ledger) -> Result<Vec<lc_reputation::Item>> {
    Ok(led.all()?.into_iter().map(|e| {
        let time = match &e.payload { Payload::Listing(l) => l.created, Payload::DealAccept(a) => a.created,
            Payload::DealStep(s) => s.created, Payload::Claim(c) => c.created, _ => now() };
        lc_reputation::Item { id: e.id, author: e.env.author, payload: e.payload, time }
    }).collect())
}

fn print_listings(led: &Ledger, f: impl Fn(&Listing) -> bool) -> Result<()> {
    let it = items(led)?;
    let rep = lc_reputation::compute(&it, now(), &Default::default());
    for i in &it {
        if let Payload::Listing(l) = &i.payload {
            if rep.get(&i.author).is_some_and(|s| s.struck) { continue; } // Proof of Trust: hide struck sellers
            if l.expiry < now() || !f(l) || led.is_blocked(&i.author)? || led.is_blocked(&pseudonym_of(&i.author))? { continue; }
            let r = rep.get(&i.author);
            println!("{} [{:?}] {} | {} | hint: {} | seller {} rep {:.2} ({} ratings, {} reports)",
                &i.id[..16], l.kind, l.title, l.category, l.price_hint, &pseudonym_of(&i.author)[..12],
                r.map(|s| s.mean).unwrap_or(0.0), r.map(|s| s.count).unwrap_or(0), r.map(|s| s.reports).unwrap_or(0));
        }
    }
    Ok(())
}

fn resolve(led: &Ledger, prefix: &str) -> Result<lc_ledger::Entry> {
    let m: Vec<_> = led.all()?.into_iter().filter(|e| e.id.starts_with(prefix)).collect();
    match m.len() { 1 => Ok(m.into_iter().next().unwrap()), 0 => bail!("no entry {prefix}"), _ => bail!("ambiguous id {prefix}") }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    std::fs::create_dir_all(&cli.dir)?;
    let led = Ledger::open(&cli.dir.join("ledger.db"), cli.stamp_bits)?;
    let seal_add = |p: Payload| -> Result<String> {
        let env = load_id(&cli.dir)?.seal(&p, cli.stamp_bits);
        led.append(&env)?;
        println!("{}", env.id());
        Ok(env.id())
    };
    match cli.cmd {
        Cmd::Keygen => {
            let p = cli.dir.join("key");
            if p.exists() { bail!("key already exists at {}", p.display()); }
            let id = Identity::generate();
            std::fs::write(&p, id.secret_bytes())?;
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?; }
            println!("pseudonym {}\npubkey    {}", id.pseudonym(), id.public_hex());
        }
        Cmd::Whoami => { let id = load_id(&cli.dir)?; println!("pseudonym {}\npubkey    {}", id.pseudonym(), id.public_hex()); }
        Cmd::Post { kind, title, desc, price_hint, category, days, image } => {
            let kind = if kind == "offer" { ListingKind::Offer } else { ListingKind::Request };
            seal_add(Payload::Listing(Listing { kind, title, desc, price_hint, category, expiry: now() + days * 86400, image_hashes: image, created: now() }))?;
        }
        Cmd::List { kind } => print_listings(&led, |l| kind.as_deref().map_or(true, |k| format!("{:?}", l.kind).eq_ignore_ascii_case(k)))?,
        Cmd::Search { query } => { let q = query.to_lowercase(); print_listings(&led, |l| format!("{} {} {}", l.title, l.desc, l.category).to_lowercase().contains(&q))? }
        Cmd::CompleteDeal { listing_id, buyer } => {
            let e = resolve(&led, &listing_id)?;
            if e.env.author != load_id(&cli.dir)?.public_hex() { bail!("only the seller of this listing can confirm a deal"); }
            seal_add(Payload::DealReceipt(DealReceipt { listing_id: e.id, buyer }))?;
        }
        Cmd::Rate { receipt_id, score, comment } => {
            let r = resolve(&led, &receipt_id)?;
            let Payload::DealReceipt(d) = &r.payload else { bail!("not a deal receipt") };
            if d.buyer != load_id(&cli.dir)?.public_hex() { bail!("this receipt names a different buyer"); }
            seal_add(Payload::Rating(Rating { listing_id: d.listing_id.clone(), seller: r.env.author.clone(), receipt_id: r.id.clone(), score, comment }))?;
        }
        Cmd::Accept { listing_id, buyer } => {
            let e = resolve(&led, &listing_id)?;
            if e.env.author != load_id(&cli.dir)?.public_hex() { bail!("only the seller can accept a buyer"); }
            seal_add(Payload::DealAccept(DealAccept { listing_id: e.id, buyer, created: now() }))?;
        }
        Cmd::Step { accept_id, step } => {
            let a = resolve(&led, &accept_id)?;
            let step = match step.as_str() { "buyer-paid" => Step::BuyerPaid, "seller-shipped" => Step::SellerShipped,
                "buyer-confirmed" => Step::BuyerConfirmed, _ => Step::BuyerDisputed };
            seal_add(Payload::DealStep(DealStep { accept_id: a.id, step, created: now() }))?;
        }
        Cmd::Claim { accept_id, reason } => {
            let a = resolve(&led, &accept_id)?;
            seal_add(Payload::Claim(Claim { accept_id: a.id, reason, created: now() }))?;
        }
        Cmd::Trust { pubkey } => {
            let rep = lc_reputation::compute(&items(&led)?, now(), &Default::default());
            println!("{:?}", rep.get(&pubkey));
        }
        Cmd::Report { target_id, reason } => { seal_add(Payload::Report(Report { target_id, reason }))?; }
        Cmd::Block { key } => { led.block(&key)?; println!("blocked {key}"); }
        Cmd::Status => println!("entries {}\nmerkle_root {}", led.len()?, led.merkle_root()?),
        Cmd::Serve { http, listen, dial } => {
            use lc_net::{Command, Event};
            use std::sync::{Arc, Mutex};
            let (ctx, crx) = tokio::sync::mpsc::channel(256);
            let (etx, mut erx) = tokio::sync::mpsc::channel(256);
            tokio::spawn(async move { if let Err(e) = lc_net::run(listen.parse().unwrap(), crx, etx).await { eprintln!("net error: {e:?}"); } });
            for d in dial { ctx.send(Command::Dial(d.parse()?)).await?; }
            let st = api::AppState { ledger: Arc::new(Mutex::new(led)), id: Arc::new(load_id(&cli.dir)?), stamp_bits: cli.stamp_bits,
                net: ctx.clone(), events: tokio::sync::broadcast::channel(256).0 };
            let tcp = tokio::net::TcpListener::bind(&http).await?;
            println!("http http://{}", tcp.local_addr()?);
            let app = api::router(st.clone());
            tokio::spawn(async move { axum::serve(tcp, app).await });
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
            let all = |st: &api::AppState| st.ledger.lock().unwrap().all().map(|v| v.into_iter().map(|e| e.env).collect::<Vec<_>>());
            loop {
                tokio::select! {
                    _ = tick.tick() => for e in all(&st)? { ctx.send(Command::Publish(e)).await?; },
                    ev = erx.recv() => match ev {
                        None => break,
                        Some(Event::Listening(a)) => println!("listening {a}"),
                        Some(Event::Connected(p)) => { println!("peer {p}"); for e in all(&st)? { ctx.send(Command::Publish(e)).await?; } }
                        Some(Event::Received(env)) => match st.ingest(env.clone()).await {
                            Ok(true) => println!("+ {}", env.id()), Ok(false) => {}, Err(e) => eprintln!("rejected: {e}"),
                        },
                    }
                }
            }
        }
        Cmd::Peers { listen, dial } => {
            use lc_net::{Command, Event};
            let (ctx, crx) = tokio::sync::mpsc::channel(256);
            let (etx, mut erx) = tokio::sync::mpsc::channel(256);
            tokio::spawn(async move { if let Err(e) = lc_net::run(listen.parse().unwrap(), crx, etx).await { eprintln!("net error: {e:?}"); } });
            for d in dial { ctx.send(Command::Dial(d.parse()?)).await?; }
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = tick.tick() => for e in led.all()? { ctx.send(Command::Publish(e.env)).await?; },
                    ev = erx.recv() => match ev {
                        None => break,
                        Some(Event::Listening(a)) => println!("listening {a}"),
                        Some(Event::Connected(p)) => { println!("peer {p}"); for e in led.all()? { ctx.send(Command::Publish(e.env)).await?; } }
                        Some(Event::Received(env)) => match led.append(&env) {
                            Ok(true) => println!("+ {}", env.id()),
                            Ok(false) => {}
                            Err(e) => eprintln!("rejected: {e}"),
                        },
                    }
                }
            }
        }
    }
    Ok(())
}
