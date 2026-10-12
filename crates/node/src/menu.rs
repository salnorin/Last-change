//! Interactive numbered text menu (plain stdin lines, Termux-friendly, no raw TTY).
//! Menu text is localized via the table below; code and docs stay English.

use anyhow::Result;
use lc_chain::LocalChain;
use lc_core::Identity;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::i18n::{t, LANGS, LANG_PROMPT};

/// Accept ASCII or Persian/Arabic-Indic digits.
fn digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '۰'..='۹' => char::from(b'0' + (c as u32 - '۰' as u32) as u8),
            '٠'..='٩' => char::from(b'0' + (c as u32 - '٠' as u32) as u8),
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn ask(lang: &str, key: &str, input: &mut impl BufRead) -> Result<Option<String>> {
    print!("{}", t(lang, key));
    std::io::stdout().flush()?;
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

/// Numbered language picker; `None` on EOF. Re-asks on invalid input.
fn pick_lang(input: &mut impl BufRead) -> Result<Option<&'static str>> {
    loop {
        println!("\n{LANG_PROMPT}");
        for (i, (_, name)) in LANGS.iter().enumerate() {
            println!("  {}) {name}", i + 1);
        }
        print!("> ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if let Ok(n) = digits(&line).parse::<usize>() {
            if (1..=LANGS.len()).contains(&n) {
                return Ok(Some(LANGS[n - 1].0));
            }
        }
    }
}

/// Run the menu. `lang = None` shows the language picker first.
/// `json = true` makes the info screens (account, explorer, send) print JSON instead of text.
pub fn run(dir: &Path, lang: Option<&str>, json: bool) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let lang = match lang {
        Some(l) => l,
        None => match pick_lang(&mut input)? {
            Some(l) => l,
            None => return Ok(()),
        },
    };
    crate::offer_migration(dir, lang)?;
    loop {
        println!("\n{}", t(lang, "title"));
        for k in ["top1", "top2", "top3", "top4", "top5", "top6"] {
            println!("  {}", t(lang, k));
        }
        let Some(choice) = ask(lang, "pj_choose", &mut input)? else {
            return Ok(());
        };
        let r: Result<bool> = match digits(&choice).as_str() {
            "1" => account(dir, lang, json, None).map(|_| true),
            "2" => wallet_menu(dir, lang, json, &mut input),
            "3" => match ask(lang, "lookup_q", &mut input)? {
                None => Ok(false),
                Some(q) => {
                    account(dir, lang, json, Some(q).filter(|q| !q.is_empty())).map(|_| true)
                }
            },
            "4" => {
                explorer_info(dir, lang, json);
                Ok(true)
            }
            "5" => {
                send_info(lang, json);
                Ok(true)
            }
            "6" => {
                println!("{}", t(lang, "bye"));
                return Ok(());
            }
            _ => {
                println!("{}", t(lang, "unknown"));
                Ok(true)
            }
        };
        match r {
            Ok(false) => return Ok(()),
            Ok(true) => {}
            Err(e) => println!("error: {e}"),
        }
    }
}

pub(crate) fn yes(s: &str) -> bool {
    let s = s.trim().to_lowercase();
    [
        "y", "yes", "j", "ja", "s", "si", "sí", "o", "oui", "e", "evet", "д", "да", "是", "ن",
        "نعم", "ب", "بله",
    ]
    .contains(&s.as_str())
}

/// Wallet section. Returns Ok(false) on EOF.
fn wallet_menu(dir: &Path, lang: &str, json: bool, input: &mut impl BufRead) -> Result<bool> {
    loop {
        println!("\n{}", t(lang, "top2"));
        for k in ["m1", "m2", "m3", "wm4", "wm5", "wm6"] {
            println!("  {}", t(lang, k));
        }
        let Some(choice) = ask(lang, "pj_choose", input)? else {
            return Ok(false);
        };
        let r: Result<()> = match digits(&choice).as_str() {
            // web-app-ready: wallet creation (words + optional secret reveal)
            "1" => {
                let Some(w) = ask(lang, "words_q", input)? else {
                    return Ok(false);
                };
                match digits(&w).parse::<usize>() {
                    Ok(n) if [12, 15, 18, 24].contains(&n) => {
                        if crate::wallet_exists(dir) {
                            let Some(a) = ask(lang, "exists_q", input)? else {
                                return Ok(false);
                            };
                            if !yes(&a) || !delete_flow(dir, lang, json, input)? {
                                continue;
                            }
                        }
                        let r = crate::wallet_new(dir, n, lang);
                        if r.is_ok() {
                            println!("{}", t(lang, "warn"));
                            let Some(a) = ask(lang, "key_q", input)? else {
                                return Ok(false);
                            };
                            if yes(&a) {
                                let id = crate::load_id(dir)?;
                                println!("{}", t(lang, "key_label"));
                                println!("{}", hex::encode(id.master_bytes()));
                                println!("{}", t(lang, "warn"));
                            }
                        }
                        r
                    }
                    _ => {
                        println!("{}", t(lang, "bad_words"));
                        Ok(())
                    }
                }
            }
            "2" => {
                let Some(w) = ask(lang, "enter_words", input)? else {
                    return Ok(false);
                };
                Identity::from_mnemonic(&w)
                    .map_err(anyhow::Error::from)
                    .and_then(|id| crate::save_wallet(dir, &id))
            }
            "3" => match crate::load_id(dir) {
                Ok(id) => {
                    crate::show_id(&id);
                    println!("{}", t(lang, "keys"));
                    let c = LocalChain::load_or_init(dir)?;
                    println!("chain tip  {} (height {})", c.tip_hash, c.height);
                    Ok(())
                }
                Err(_) => {
                    println!("{}", t(lang, "no_wallet"));
                    Ok(())
                }
            },
            "4" => delete_flow(dir, lang, json, input).map(|_| ()),
            "5" => paper(dir, lang, input),
            "6" => return Ok(true),
            _ => {
                println!("{}", t(lang, "unknown"));
                Ok(())
            }
        };
        if let Err(e) = r {
            println!("error: {e}");
        }
    }
}

/// Own pseudonym without asking for the passphrase (public field of the keystore).
fn own_pseudonym(dir: &Path) -> Option<String> {
    crate::wallet_pseudonym(dir).or_else(|| crate::load_id(dir).ok().map(|i| i.pseudonym()))
}

/// web-app-ready: delete flow. Warn, require last 4 chars of the pseudonym, then move
/// the folder to a backup. Returns Ok(true) if moved.
fn delete_flow(dir: &Path, lang: &str, json: bool, input: &mut impl BufRead) -> Result<bool> {
    let Some(p) = own_pseudonym(dir) else {
        println!("{}", t(lang, "no_wallet"));
        return Ok(false);
    };
    let code = &p[p.len() - 4..];
    println!("{}\n{p}", t(lang, "del_warn"));
    let Some(a) = ask(lang, "del_q", input)? else {
        return Ok(false);
    };
    if a.trim().to_lowercase() != code.to_lowercase() {
        println!("{}", t(lang, "del_cancel"));
        return Ok(false);
    }
    let (to, files) = crate::wallet_delete(dir)?;
    if json {
        println!("{}", serde_json::json!({"moved_to": to, "files": files}));
    } else {
        println!("{} {}", t(lang, "del_done"), to.display());
        for f in &files {
            println!("  {f}");
        }
    }
    std::fs::create_dir_all(dir)?;
    Ok(true)
}

/// web-app-ready: read-only account view (pseudonym, ledger history, reputation). No value logic.
/// `query = None` means the own wallet.
fn account(dir: &Path, lang: &str, json: bool, query: Option<String>) -> Result<()> {
    let (pseudonym, pubkey) = match &query {
        Some(q) => (q.clone(), q.clone()),
        None => match own_pseudonym(dir) {
            Some(p) => (p.clone(), p),
            None => {
                println!("{}", t(lang, "no_wallet"));
                return Ok(());
            }
        },
    };
    let led = lc_ledger::Ledger::open(&dir.join("ledger.db"), 0)?;
    let items = crate::items(&led)?;
    let mine: Vec<_> = items
        .iter()
        .filter(|i| i.pseudonym == pseudonym || i.author == pubkey)
        .collect();
    let author = mine.first().map(|i| i.author.clone()).unwrap_or(pubkey);
    let pseudonym = mine
        .first()
        .map(|i| i.pseudonym.clone())
        .unwrap_or(pseudonym);
    let rep = lc_reputation::compute(&items, crate::now(), &Default::default());
    let score = rep.get(&author);
    let kind = |i: &lc_reputation::Item| {
        format!("{:?}", i.payload)
            .split(['(', ' ', '{'])
            .next()
            .unwrap_or("")
            .to_string()
    };
    if json {
        let hist: Vec<_> = mine
            .iter()
            .map(|i| serde_json::json!({"id": i.id, "kind": kind(i), "time": i.time}))
            .collect();
        let v = serde_json::json!({
            "query": query,
            "pseudonym": pseudonym,
            "pubkey": author,
            "holdings": 0,
            "note": "testnet has no coins or on-chain value",
            "history": hist,
            "reputation": score.map(|s| serde_json::json!({
                "mean": s.mean, "count": s.count, "reports": s.reports, "strikes": s.strikes,
            })),
        });
        println!("{v}");
    } else {
        println!("\npseudonym  {pseudonym}\npubkey     {author}\nholdings   0");
        println!("{}", t(lang, "no_coin"));
        match score {
            Some(s) => println!(
                "reputation {:.2} ({} ratings, {} reports, {} strikes)",
                s.mean, s.count, s.reports, s.strikes
            ),
            None => println!("reputation -"),
        }
        println!("{}", t(lang, "history"));
        if mine.is_empty() {
            println!("{}", t(lang, "no_history"));
        }
        for i in &mine {
            println!("  {}  {}  {}", i.time, kind(i), &i.id[..i.id.len().min(16)]);
        }
    }
    Ok(())
}

/// web-app-ready: explorer pointer + local chain summary.
fn explorer_info(dir: &Path, lang: &str, json: bool) {
    let c = LocalChain::load_or_init(dir).ok();
    let full = cfg!(feature = "full");
    if json {
        let v = serde_json::json!({
            "explorer_in_this_build": full,
            "command": "lastchange explorer --http 127.0.0.1:8090",
            "url": "http://127.0.0.1:8090",
            "api": ["/api/blocks", "/api/block/:hash"],
            "height": c.as_ref().map(|c| c.height),
            "tip_hash": c.as_ref().map(|c| c.tip_hash.clone()),
        });
        println!("{v}");
    } else {
        println!("{}", t(lang, "explorer_info"));
        if let Some(c) = c {
            println!("chain tip  {} (height {})", c.tip_hash, c.height);
        }
    }
}

/// web-app-ready: sending is intentionally unavailable (no on-chain value exists).
fn send_info(lang: &str, json: bool) {
    if json {
        let v = serde_json::json!({
            "available": false,
            "reason": "Last-change testnet has no coins. See docs/FUTURE_POW_TOKEN.md",
        });
        println!("{v}");
    } else {
        println!("{}", t(lang, "send_na"));
    }
}

fn num(s: &str) -> Option<f64> {
    let s = digits(s).replace(['٫', ','], ".");
    if s.is_empty() {
        Some(0.0)
    } else {
        s.parse().ok()
    }
}

fn paper(dir: &Path, lang: &str, input: &mut impl BufRead) -> Result<()> {
    use crate::journal::{unrealized, Journal, Side};
    let j = Journal::open(dir)?;
    loop {
        println!("\n{}\n{}", t(lang, "pj_title"), t(lang, "pj_menu"));
        let Some(c) = ask(lang, "pj_choose", input)? else {
            return Ok(());
        };
        match digits(&c).as_str() {
            "1" | "2" => {
                let side = if digits(&c) == "1" {
                    Side::Buy
                } else {
                    Side::Sell
                };
                let Some(asset) = ask(lang, "pj_asset", input)? else {
                    return Ok(());
                };
                let Some(q) = ask(lang, "pj_qty", input)? else {
                    return Ok(());
                };
                let Some(p) = ask(lang, "pj_price", input)? else {
                    return Ok(());
                };
                let Some(f) = ask(lang, "pj_fee", input)? else {
                    return Ok(());
                };
                let Some(note) = ask(lang, "pj_note", input)? else {
                    return Ok(());
                };
                match (num(&q), num(&p), num(&f)) {
                    (Some(q), Some(p), Some(f)) => {
                        match j.add(crate::now(), side, &asset, q, p, f, &note) {
                            Ok(_) => println!("{}", t(lang, "pj_saved")),
                            Err(e) => println!("error: {e}"),
                        }
                    }
                    _ => println!("error: bad number"),
                }
            }
            "3" => {
                let pos = j.positions()?;
                for (a, p) in &pos {
                    print!(
                        "{a}: qty {:.8} avg {:.4} realized {:.4}",
                        p.qty, p.avg_cost, p.realized
                    );
                    if p.qty > 0.0 {
                        println!();
                        if let Some(cur) = ask(lang, "pj_cur", input)? {
                            if let Some(c) = num(&cur).filter(|c| *c > 0.0) {
                                println!("  unrealized {:.4}", unrealized(p, c));
                            }
                        }
                    } else {
                        println!();
                    }
                }
                let total: f64 = pos.values().map(|p| p.realized).sum();
                println!("total realized (paper) {total:.4}");
            }
            "4" => {
                for tr in j.trades()? {
                    println!(
                        "#{} {:?} {} {} @ {} fee {} {}",
                        tr.id, tr.side, tr.qty, tr.asset, tr.price, tr.fee, tr.note
                    );
                }
            }
            "5" => {
                let path = dir.join("paper_trades.csv");
                std::fs::write(&path, j.csv()?)?;
                println!("{} {}", t(lang, "pj_csv"), path.display());
            }
            _ => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persian_digits() {
        assert_eq!(digits("۲۴"), "24");
        assert_eq!(digits(" 3 "), "3");
    }
}
