//! `lastchange` CLI. Data lives in a local directory (default ./.lastchange):
//! `key` (32-byte ed25519 secret, nothing else) and `ledger.db`.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use lc_chain::LocalChain;
use lc_core::*;
use lc_ledger::Ledger;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "full")]
mod api;
#[cfg(feature = "full")]
mod explorer;

#[derive(Parser)]
#[command(
    name = "lastchange",
    about = "Pseudonymous P2P classifieds with reputation."
)]
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
    /// Alias for `wallet new`.
    Keygen,
    /// Show your pseudonym and public keys.
    Whoami,
    /// Identity keystore: hybrid Ed25519 + ML-DSA-65 keys, Argon2id-encrypted. No balances.
    Wallet {
        #[command(subcommand)]
        cmd: WalletCmd,
    },
    /// Encrypt a deal message to a peer (hybrid X25519 + ML-KEM-768).
    MsgSeal {
        #[arg(long)]
        to_x25519: String,
        #[arg(long)]
        to_mlkem: String,
        #[arg(long, default_value = "")]
        aad: String,
        text: String,
    },
    /// Decrypt a deal message (JSON from msg-seal).
    MsgOpen {
        #[arg(long, default_value = "")]
        aad: String,
        sealed_json: String,
    },
    /// Post an offer or request.
    Post {
        #[arg(value_parser = ["offer", "request"])]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "")]
        desc: String,
        #[arg(long, default_value = "")]
        price_hint: String,
        #[arg(long, default_value = "misc")]
        category: String,
        #[arg(long, default_value_t = 30)]
        days: u64,
        #[arg(long)]
        image: Vec<String>,
    },
    /// List active listings with seller reputation.
    List {
        #[arg(long)]
        kind: Option<String>,
    },
    /// Search listings by text.
    Search { query: String },
    /// Seller: confirm a completed deal with a buyer (enables their rating).
    CompleteDeal {
        listing_id: String,
        #[arg(long)]
        buyer: String,
    },
    /// Buyer: rate a seller after receiving, referencing the seller's receipt.
    Rate {
        receipt_id: String,
        #[arg(long)]
        score: u8,
        #[arg(long, default_value = "")]
        comment: String,
    },
    /// Seller: accept a buyer for a listing (Proof of Trust step 1).
    Accept {
        listing_id: String,
        #[arg(long)]
        buyer: String,
    },
    /// Record a lifecycle step: buyer-paid | seller-shipped | buyer-confirmed | buyer-disputed.
    Step {
        accept_id: String,
        #[arg(value_parser = ["buyer-paid", "seller-shipped", "buyer-confirmed", "buyer-disputed"])]
        step: String,
    },
    /// Buyer: non-delivery claim after the timeout (costs a stronger stamp; false claims hurt you).
    Claim {
        accept_id: String,
        #[arg(long, default_value = "not delivered")]
        reason: String,
    },
    /// Show trust status (score, strikes, false claims) of a pubkey.
    Trust { pubkey: String },
    /// Flag a listing or author for others (gossiped).
    Report {
        target_id: String,
        #[arg(long)]
        reason: String,
    },
    /// Locally block a pseudonym, pubkey or entry id.
    Block { key: String },
    /// Show ledger stats and Merkle root.
    Status,
    /// Local block explorer (genesis + tip, circuit details).
    #[cfg(feature = "full")]
    Explorer {
        #[arg(long, default_value = "127.0.0.1:8090")]
        http: String,
    },
    /// Run P2P node + local HTTP/JSON/WebSocket API and web app.
    #[cfg(feature = "full")]
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        http: String,
        #[arg(long, default_value = "/ip4/0.0.0.0/tcp/0")]
        listen: String,
        #[arg(long)]
        dial: Vec<String>,
    },
    /// Run a P2P node: sync local entries and receive peers' entries.
    #[cfg(feature = "full")]
    Peers {
        #[arg(long, default_value = "/ip4/0.0.0.0/tcp/0")]
        listen: String,
        #[arg(long)]
        dial: Vec<String>,
    },
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn passphrase(confirm: bool) -> Result<String> {
    if let Ok(p) = std::env::var("LASTCHANGE_PASSPHRASE") {
        return Ok(p);
    }
    let p = rpassword::prompt_password("Keystore passphrase: ")?;
    if confirm && rpassword::prompt_password("Repeat passphrase: ")? != p {
        bail!("passphrases differ");
    }
    Ok(p)
}

fn keystore_path(dir: &std::path::Path) -> PathBuf {
    dir.join("keystore.json")
}

fn load_id(dir: &std::path::Path) -> Result<Identity> {
    let s = std::fs::read_to_string(keystore_path(dir))
        .context("no keystore; run `lastchange wallet new`")?;
    let ks: keystore::Keystore = serde_json::from_str(&s)?;
    Ok(ks.decrypt(&passphrase(false)?)?)
}

fn wallet_new(dir: &std::path::Path) -> Result<()> {
    let p = keystore_path(dir);
    if p.exists() {
        bail!("keystore already exists at {}", p.display());
    }
    let id = Identity::generate();
    let ks = keystore::Keystore::encrypt(&id, &passphrase(true)?)?;
    std::fs::write(&p, serde_json::to_string_pretty(&ks)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
    }
    let chain = LocalChain::from_genesis();
    chain.save(dir)?;
    show_id(&id);
    println!("chain tip  {} (height {})", chain.tip_hash, chain.height);
    println!("genesis    {}", chain.genesis_hash);
    Ok(())
}

fn show_id(id: &Identity) {
    let k = id.kem_public();
    println!("pseudonym     {}\ned25519       {}\nml-dsa-65     {}… ({} bytes)\nx25519        {}\nml-kem-768    {}… ({} bytes)",
        id.pseudonym(), id.public_hex(), &id.pq_public_hex()[..32], id.pq_public_hex().len() / 2, k.x25519, &k.mlkem768[..32], k.mlkem768.len() / 2);
}

pub(crate) fn items(led: &Ledger) -> Result<Vec<lc_reputation::Item>> {
    Ok(led
        .all()?
        .into_iter()
        .map(|e| {
            let time = match &e.payload {
                Payload::Listing(l) => l.created,
                Payload::DealAccept(a) => a.created,
                Payload::DealStep(s) => s.created,
                Payload::Claim(c) => c.created,
                _ => now(),
            };
            lc_reputation::Item {
                id: e.id,
                pseudonym: e.env.author_pseudonym(),
                author: e.env.author,
                payload: e.payload,
                time,
            }
        })
        .collect())
}

fn print_listings(led: &Ledger, f: impl Fn(&Listing) -> bool) -> Result<()> {
    let it = items(led)?;
    let rep = lc_reputation::compute(&it, now(), &Default::default());
    for i in &it {
        if let Payload::Listing(l) = &i.payload {
            if rep.get(&i.author).is_some_and(|s| s.struck) {
                continue;
            } // Proof of Trust: hide struck sellers
            if l.expiry < now()
                || !f(l)
                || led.is_blocked(&i.author)?
                || led.is_blocked(&i.pseudonym)?
            {
                continue;
            }
            let r = rep.get(&i.author);
            println!(
                "{} [{:?}] {} | {} | hint: {} | seller {} rep {:.2} ({} ratings, {} reports)",
                &i.id[..16],
                l.kind,
                l.title,
                l.category,
                l.price_hint,
                &i.pseudonym[..12],
                r.map(|s| s.mean).unwrap_or(0.0),
                r.map(|s| s.count).unwrap_or(0),
                r.map(|s| s.reports).unwrap_or(0)
            );
        }
    }
    Ok(())
}

fn resolve(led: &Ledger, prefix: &str) -> Result<lc_ledger::Entry> {
    let m: Vec<_> = led
        .all()?
        .into_iter()
        .filter(|e| e.id.starts_with(prefix))
        .collect();
    match m.len() {
        1 => Ok(m.into_iter().next().unwrap()),
        0 => bail!("no entry {prefix}"),
        _ => bail!("ambiguous id {prefix}"),
    }
}

#[derive(Subcommand)]
enum WalletCmd {
    /// Create a new identity keystore (asks for a passphrase or uses LASTCHANGE_PASSPHRASE).
    New,
    /// Show public keys and pseudonym.
    Show,
    /// Print public keys as JSON plus the ENCRYPTED keystore for backup.
    Export,
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
        Cmd::Keygen
        | Cmd::Wallet {
            cmd: WalletCmd::New,
        } => wallet_new(&cli.dir)?,
        Cmd::Whoami
        | Cmd::Wallet {
            cmd: WalletCmd::Show,
        } => {
            show_id(&load_id(&cli.dir)?);
            let chain = LocalChain::load_or_init(&cli.dir)?;
            println!("chain tip  {} (height {})", chain.tip_hash, chain.height);
            println!("genesis    {}", chain.genesis_hash);
        }
        Cmd::Wallet {
            cmd: WalletCmd::Export,
        } => {
            let id = load_id(&cli.dir)?;
            let ks: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(keystore_path(&cli.dir))?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"pseudonym": id.pseudonym(), "ed25519": id.public_hex(),
                "ml_dsa_65": id.pq_public_hex(), "kem": id.kem_public(), "encrypted_keystore": ks})
                )?
            );
        }
        Cmd::MsgSeal {
            to_x25519,
            to_mlkem,
            aad,
            text,
        } => {
            let s = channel::seal_to(
                &channel::KemPublic {
                    x25519: to_x25519,
                    mlkem768: to_mlkem,
                },
                text.as_bytes(),
                aad.as_bytes(),
            )?;
            println!("{}", serde_json::to_string(&s)?);
        }
        Cmd::MsgOpen { aad, sealed_json } => {
            let s: channel::Sealed = serde_json::from_str(&sealed_json)?;
            println!(
                "{}",
                String::from_utf8_lossy(&load_id(&cli.dir)?.open(&s, aad.as_bytes())?)
            );
        }
        Cmd::Post {
            kind,
            title,
            desc,
            price_hint,
            category,
            days,
            image,
        } => {
            let kind = if kind == "offer" {
                ListingKind::Offer
            } else {
                ListingKind::Request
            };
            seal_add(Payload::Listing(Listing {
                kind,
                title,
                desc,
                price_hint,
                category,
                expiry: now() + days * 86400,
                image_hashes: image,
                created: now(),
            }))?;
        }
        Cmd::List { kind } => print_listings(&led, |l| {
            kind.as_deref()
                .is_none_or(|k| format!("{:?}", l.kind).eq_ignore_ascii_case(k))
        })?,
        Cmd::Search { query } => {
            let q = query.to_lowercase();
            print_listings(&led, |l| {
                format!("{} {} {}", l.title, l.desc, l.category)
                    .to_lowercase()
                    .contains(&q)
            })?
        }
        Cmd::CompleteDeal { listing_id, buyer } => {
            let e = resolve(&led, &listing_id)?;
            if e.env.author != load_id(&cli.dir)?.public_hex() {
                bail!("only the seller of this listing can confirm a deal");
            }
            seal_add(Payload::DealReceipt(DealReceipt {
                listing_id: e.id,
                buyer,
            }))?;
        }
        Cmd::Rate {
            receipt_id,
            score,
            comment,
        } => {
            let r = resolve(&led, &receipt_id)?;
            let Payload::DealReceipt(d) = &r.payload else {
                bail!("not a deal receipt")
            };
            if d.buyer != load_id(&cli.dir)?.public_hex() {
                bail!("this receipt names a different buyer");
            }
            seal_add(Payload::Rating(Rating {
                listing_id: d.listing_id.clone(),
                seller: r.env.author.clone(),
                receipt_id: r.id.clone(),
                score,
                comment,
            }))?;
        }
        Cmd::Accept { listing_id, buyer } => {
            let e = resolve(&led, &listing_id)?;
            if e.env.author != load_id(&cli.dir)?.public_hex() {
                bail!("only the seller can accept a buyer");
            }
            seal_add(Payload::DealAccept(DealAccept {
                listing_id: e.id,
                buyer,
                created: now(),
            }))?;
        }
        Cmd::Step { accept_id, step } => {
            let a = resolve(&led, &accept_id)?;
            let step = match step.as_str() {
                "buyer-paid" => Step::BuyerPaid,
                "seller-shipped" => Step::SellerShipped,
                "buyer-confirmed" => Step::BuyerConfirmed,
                _ => Step::BuyerDisputed,
            };
            seal_add(Payload::DealStep(DealStep {
                accept_id: a.id,
                step,
                created: now(),
            }))?;
        }
        Cmd::Claim { accept_id, reason } => {
            let a = resolve(&led, &accept_id)?;
            seal_add(Payload::Claim(Claim {
                accept_id: a.id,
                reason,
                created: now(),
            }))?;
        }
        Cmd::Trust { pubkey } => {
            let rep = lc_reputation::compute(&items(&led)?, now(), &Default::default());
            println!("{:?}", rep.get(&pubkey));
        }
        Cmd::Report { target_id, reason } => {
            seal_add(Payload::Report(Report { target_id, reason }))?;
        }
        Cmd::Block { key } => {
            led.block(&key)?;
            println!("blocked {key}");
        }
        Cmd::Status => println!("entries {}\nmerkle_root {}", led.len()?, led.merkle_root()?),
        #[cfg(feature = "full")]
        Cmd::Explorer { http } => {
            let _ = LocalChain::load_or_init(&cli.dir)?;
            explorer::serve(&cli.dir, &http).await?;
        }
        #[cfg(feature = "full")]
        Cmd::Serve { http, listen, dial } => {
            use lc_net::{Command, Event};
            use std::sync::{Arc, Mutex};
            let (ctx, crx) = tokio::sync::mpsc::channel(256);
            let (etx, mut erx) = tokio::sync::mpsc::channel(256);
            tokio::spawn(async move {
                if let Err(e) = lc_net::run(listen.parse().unwrap(), crx, etx).await {
                    eprintln!("net error: {e:?}");
                }
            });
            for d in dial {
                ctx.send(Command::Dial(d.parse()?)).await?;
            }
            let st = api::AppState {
                ledger: Arc::new(Mutex::new(led)),
                id: Arc::new(load_id(&cli.dir)?),
                stamp_bits: cli.stamp_bits,
                net: ctx.clone(),
                events: tokio::sync::broadcast::channel(256).0,
            };
            let tcp = tokio::net::TcpListener::bind(&http).await?;
            println!("http http://{}", tcp.local_addr()?);
            let app = api::router(st.clone());
            tokio::spawn(async move { axum::serve(tcp, app).await });
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
            let all = |st: &api::AppState| {
                st.ledger
                    .lock()
                    .unwrap()
                    .all()
                    .map(|v| v.into_iter().map(|e| e.env).collect::<Vec<_>>())
            };
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
        #[cfg(feature = "full")]
        Cmd::Peers { listen, dial } => {
            use lc_net::{Command, Event};
            let (ctx, crx) = tokio::sync::mpsc::channel(256);
            let (etx, mut erx) = tokio::sync::mpsc::channel(256);
            tokio::spawn(async move {
                if let Err(e) = lc_net::run(listen.parse().unwrap(), crx, etx).await {
                    eprintln!("net error: {e:?}");
                }
            });
            for d in dial {
                ctx.send(Command::Dial(d.parse()?)).await?;
            }
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
