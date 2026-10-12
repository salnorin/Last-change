# Changelog

## Unreleased
- BIP39 wallet: `wallet new [--words 12|24] [--passphrase]`, `wallet restore`. No prompt by default.
- Testnet genesis (zero value) with founding message and dedication; `wallet new/show` show chain tip.
- `lastchange explorer` (/blocks, /block/:hash, circuit details). GitHub Pages site. `scripts/install.sh`, docs/TERMUX.md, release binaries (Linux, musl aarch64, Android, macOS, Windows).
- Web app split into `web/css` + `web/js` modules (no inline scripts); mobile-friendly layout.
- Docs index (`docs/README.md`), per-crate READMEs, `.editorconfig`.
- CI: `cargo fmt --check` and `clippy -D warnings`.

## 0.1.0 — 2026-10-12
- Pseudonymous P2P marketplace (offers/requests, art category).
- Proof of Trust deal lifecycle with one-strike policy.
- circuit-pow anti-spam stamps; libp2p gossip; SQLite Merkle ledger.
- Hybrid post-quantum signatures (Ed25519 + ML-DSA-65) and ML-KEM-768 deal channel.
- Argon2id keystore (`wallet new/show/export`); client-sign / node-stamp envelopes.
- Local axum API + web app; content filter hard floor; value-logic CI guard.
- EST token / mining rewards: design-only, not implemented.
