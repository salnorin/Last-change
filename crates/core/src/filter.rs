//! Local content filter. Every node applies it before storing or relaying.
//! The list is intentionally conservative and can be extended per node.

const FORBIDDEN: &[&str] = &[
    "firearm", "weapon", "ammunition", "explosive", "cocaine", "heroin", "fentanyl",
    "methamphetamine", "counterfeit", "fake passport", "fake id", "stolen", "hitman",
    "child", "malware", "ransomware", "credit card dump", "waffe", "drogen", "falschgeld",
    "اسلحه", "مواد مخدر", "جعلی",
];

pub fn is_forbidden(text: &str) -> bool {
    let t = text.to_lowercase();
    FORBIDDEN.iter().any(|w| t.contains(w))
}
