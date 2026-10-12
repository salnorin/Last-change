//! Local content filter applied by every node before storing or relaying.
//!
//! HARD_FLOOR is a permanent, code-level ban list that no vote, config or
//! peer can relax: child sexual abuse / anything sexualizing or endangering
//! minors, weapons/explosives, hard drugs, human trafficking. Governance and
//! node config may only ADD terms (see `is_forbidden_with`).

pub const HARD_FLOOR: &[&str] = &[
    // minors / CSAM
    "child", "minor", "underage", "kinder", "csam", "pedo", "loli", "کودک", "خردسال",
    // weapons / explosives
    "firearm", "weapon", "ammunition", "explosive", "grenade", "waffe", "sprengstoff", "اسلحه", "مواد منفجره",
    // hard drugs
    "cocaine", "heroin", "fentanyl", "methamphetamine", "crystal meth", "drogen", "مواد مخدر",
    // human trafficking
    "human trafficking", "menschenhandel", "قاچاق انسان",
];

/// Other illegal categories blocked by default.
const DEFAULT_EXTRA: &[&str] = &[
    "counterfeit", "fake passport", "fake id", "stolen", "hitman", "malware", "ransomware",
    "credit card dump", "falschgeld", "جعلی",
];

pub fn is_forbidden(text: &str) -> bool { is_forbidden_with(text, &[]) }

/// `extra` can only add restrictions; the hard floor is always applied.
pub fn is_forbidden_with(text: &str, extra: &[String]) -> bool {
    let t = text.to_lowercase();
    HARD_FLOOR.iter().chain(DEFAULT_EXTRA).any(|w| t.contains(w)) || extra.iter().any(|w| t.contains(&w.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hard_floor_cannot_be_relaxed() {
        assert!(is_forbidden_with("selling explosives", &[]));
        assert!(is_forbidden_with("heroin", &["nothing".into()]));
        assert!(is_forbidden_with("bike", &["bike".into()]));
        assert!(!is_forbidden("old bicycle"));
    }
}
