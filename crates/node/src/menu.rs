//! Interactive numbered text menu (plain stdin lines, Termux-friendly, no raw TTY).
//! Menu text is localized via the table below; code and docs stay English.

use anyhow::Result;
use lc_chain::LocalChain;
use lc_core::Identity;
use std::io::{BufRead, Write};
use std::path::Path;

/// (key, English, Persian)
const T: &[(&str, &str, &str)] = &[
    ("title", "Last-change wallet", "کیف پول Last-change"),
    ("m1", "1) New wallet", "۱) کیف پول جدید"),
    ("m2", "2) Restore from words", "۲) بازیابی با کلمات"),
    ("m3", "3) Show wallet info", "۳) نمایش اطلاعات کیف پول"),
    ("m4", "4) Exit", "۴) خروج"),
    ("choose", "Choose (1-4): ", "انتخاب کنید (۱ تا ۴): "),
    ("words_q", "How many words? 12 / 15 / 18 / 24 (24 recommended for post-quantum margin; 16 is not a BIP39 length): ",
     "چند کلمه؟ ۱۲ / ۱۵ / ۱۸ / ۲۴ (۲۴ برای حاشیه‌ی امنیت پساکوانتومی توصیه می‌شود؛ ۱۶ طول معتبر BIP39 نیست): "),
    ("bad_words", "Invalid length. Use 12, 15, 18 or 24.", "طول نامعتبر است. ۱۲، ۱۵، ۱۸ یا ۲۴ را وارد کنید."),
    ("enter_words", "Enter your words separated by spaces: ", "کلمات خود را با فاصله وارد کنید: "),
    ("no_wallet", "No wallet yet. Choose 1 or 2.", "هنوز کیف پولی نیست. ۱ یا ۲ را انتخاب کنید."),
    ("keys", "Key types: Ed25519 + ML-DSA-65 (FIPS 204) hybrid signatures; X25519 + ML-KEM-768 (FIPS 203) encryption. All derived from your words.",
     "نوع کلیدها: امضای ترکیبی Ed25519 + ML-DSA-65 (FIPS 204)؛ رمزنگاری X25519 + ML-KEM-768 (FIPS 203). همه از کلمات شما ساخته می‌شوند."),
    ("bye", "Bye.", "خدانگهدار."),
    ("unknown", "Unknown choice.", "گزینه‌ی نامعتبر."),
];

fn t(lang: &str, key: &str) -> &'static str {
    let row = T.iter().find(|r| r.0 == key).expect("i18n key");
    if lang == "en" {
        row.1
    } else {
        row.2
    }
}

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

pub fn run(dir: &Path, lang: &str) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        println!("\n{}", t(lang, "title"));
        for k in ["m1", "m2", "m3", "m4"] {
            println!("  {}", t(lang, k));
        }
        let Some(choice) = ask(lang, "choose", &mut input)? else {
            return Ok(());
        };
        let r: Result<()> = match digits(&choice).as_str() {
            "1" => {
                let Some(w) = ask(lang, "words_q", &mut input)? else {
                    return Ok(());
                };
                match digits(&w).parse::<usize>() {
                    Ok(n) if [12, 15, 18, 24].contains(&n) => crate::wallet_new(dir, n, false),
                    _ => {
                        println!("{}", t(lang, "bad_words"));
                        Ok(())
                    }
                }
            }
            "2" => {
                let Some(w) = ask(lang, "enter_words", &mut input)? else {
                    return Ok(());
                };
                Identity::from_mnemonic(&w)
                    .map_err(anyhow::Error::from)
                    .and_then(|id| crate::save_wallet(dir, &id, false))
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
            "4" => {
                println!("{}", t(lang, "bye"));
                return Ok(());
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persian_digits() {
        assert_eq!(digits("۲۴"), "24");
        assert_eq!(digits(" 3 "), "3");
    }
    #[test]
    fn all_keys_have_both_languages() {
        for (k, en, fa) in T {
            assert!(!en.is_empty() && !fa.is_empty(), "{k}");
        }
    }
}
