# Last-change

**A pseudonymous peer-to-peer marketplace with no middleman and provable trust.**

> Bitcoin removed the third party from money. Last-change removes it from buying and selling.
>
> **No third party.** Any two pseudonymous parties trade directly with each other. Neither needs to learn the other's real identity, and no platform sits in the middle of the deal.

> ### ⚖️ Legal notice
> - This software has **no token, no cryptocurrency, no payments, no escrow and never holds funds.** Money and goods change hands directly between the parties, outside the software.
> - Adding a token, mining rewards, custody or a swap service could be a licensed activity in Germany/EU (**BaFin, KWG, MiCA**). The **EU AML Regulation (2024/1624)**, from **July 2027**, also bars crypto-asset service providers from handling anonymity-enhancing coins and anonymous accounts. Get German legal counsel before implementing any of that.
> - **Illegal goods and services are forbidden.** Pseudonymity does not exempt anyone from the law ([TERMS.md](TERMS.md)). Some categories (CSAM / anything endangering minors, weapons and explosives, hard drugs, human trafficking) are permanently banned at code level.
> - CI ([`scripts/check_no_token.sh`](scripts/check_no_token.sh)) ensures that no balances, rewards, minting, transfers, payments, swaps, exchange, escrow or custody logic enters the live code. *Policy change:* a zero-value **testnet** miner is allowed (coinbase value is always 0, asserted in tests). Proof-of-work alone is not a regulated activity, but issuing or transferring value can be.

## Features (implemented)
- **Post-quantum hybrid signatures:** every message is signed with **Ed25519 + ML-DSA-65 (FIPS 204)**, and both must verify. Private deal messages use **X25519 + ML-KEM-768 (FIPS 203)**. Keys live in an Argon2id-encrypted keystore (`lastchange wallet new/show/export`, keys only, no balances). Trade-off: about 3.3 KB signatures, so entries are about 25× larger. The libp2p transport itself is not PQ yet ([docs/POST_QUANTUM.md](docs/POST_QUANTUM.md)).
- **Pseudonymous P2P:** your identity is a keypair generated on your device. There is no name, email, phone, KYC or profile. Any two parties trade directly; neither learns the other's real identity. Networking runs on libp2p (gossipsub, Kademlia, Noise), and the network key is kept separate from your identity.
- **Offers and requests:** goods, services, and **artworks/paintings** (`category=art`) with image content-hash links. Images stay off the ledger.
- **Proof of Trust:** a signed deal lifecycle: accept, paid (off-network), shipped, and then confirmed or disputed. If the seller accepted but posted no "shipped" within the timeout (14 days by default), the buyer's non-delivery claim gives the seller a **strike**. The default is a **one-strike** policy (configurable): honest peers hide that seller's listings and zero their reputation. A false claim halves the claimer's own rating weight, and every claim needs a 16× harder anti-spam stamp.
- **Fake-review-resistant reputation:** you can rate only with a seller-signed "deal completed" receipt. Ratings are weighted by the rater's reputation and decay over time.
- **Anti-Sybil / anti-spam:** every message carries a small work stamp from **circuit-pow**. A seed generates a bounded random circuit (integer ops, branches, a scratchpad), and its output goes through a SHA-256/SHA-512/SHA3-256/BLAKE3 chain whose order is permuted per seed. The design is ASIC-resistant and uses LWMA retargeting. This is **not mining** and creates no value.
- **Moderation:** a local filter with a non-relaxable hard floor, blocklists (key, pseudonym or entry) and gossiped reports.
- **Local API + web app:** HTTP/JSON + WebSocket (axum), and a mobile-friendly web app for posting, browsing, deals and ratings.

## Roadmap (not implemented yet)
- 🔜 A ZK layer (STARK) for anonymous ratings and governance votes
- 🔜 An Android app (Rust core via uniffi) and a web3 front end that signs with the user's own key ([docs/CLIENTS.md](docs/CLIENTS.md))
- 🚫 **EST token:** an optional mined test unit. **Design only, not implemented, no value** ([docs/FUTURE_POW_TOKEN.md](docs/FUTURE_POW_TOKEN.md)).
- 🚫 **Monero / delisted-asset atomic swaps:** **design only, not implemented.** They need legal review first.

## Quick start
Website: https://salnorin.github.io/Last-change/ (testnet explorer and UI mock-ups)

One-line install (Linux/macOS/Termux, needs Rust):
```sh
curl -sSf https://raw.githubusercontent.com/salnorin/Last-change/main/scripts/install.sh | sh
```
Android/Termux: [docs/TERMUX.md](docs/TERMUX.md). Prebuilt binaries come from the `release` workflow.

```sh
cargo build --release && B=target/release/lastchange
$B wallet new --words 24          # write the words down
$B post offer --title "Oil painting" --category art --image <content-hash>
$B serve --http 127.0.0.1:8080             # web app at http://127.0.0.1:8080
$B --dir node2 keygen && $B --dir node2 serve --http 127.0.0.1:8081 --dial /ip4/127.0.0.1/tcp/<port>
./scripts/e2e.sh                            # full two-node demo/test
cargo test --workspace
```
CLI: `wallet new|show|export explorer msg-seal msg-open keygen whoami post list search accept step claim complete-deal rate trust report block status peers serve`

## Testnet genesis
`lastchange wallet new` also creates local chain state at the hardcoded testnet genesis
(`crates/chain/genesis.hash`). Its spendable coinbase value is 0, and it carries the founding message plus a symbolic **50 EST locked forever, spendable by no one** (a burn script with no key, and a consensus rule rejects any reference to it). `lastchange explorer` shows
blocks and circuit details (gate count, hash order) at http://127.0.0.1:8090.

## Layout
`crates/circuit-pow` (work stamp) · `core` (types, signing, filter) · `ledger` (SQLite + Merkle root) · `reputation` (scores + Proof of Trust) · `net` (libp2p) · `node` (CLI, API, web app) · `web/`

## Author
**Salar Khanbaba (salnorin)**. See also [MISSION.md](MISSION.md) and [docs/FUTURE.md](docs/FUTURE.md).

*For Satoshi Nakamoto, who stepped out of the light so that no one would need to be trusted, and showed that power shared by everyone belongs to no one.*

## Wallet words (BIP39)
`lastchange wallet new` prints **12 BIP39 words** once (use `--words 24`, recommended for post-quantum
margin). Write them down. `lastchange wallet restore "<words>"` restores the same identity. All keys (Ed25519,
ML-DSA-65, X25519, ML-KEM-768) are derived from the BIP39 seed with domain-separated HKDF. The words are shown
once and **never stored**. The key is stored only in `keystore.json`, encrypted with Argon2id + XChaCha20-Poly1305
under a **required passphrase** (min. 8 chars, asked twice; hidden prompt works in Termux, or set
`LASTCHANGE_PASSPHRASE`). Write the words on paper, never screenshot or share them: lost words = wallet gone forever.
`--passphrase` is now a no-op kept for compatibility.

**Old plaintext `seed.json`?** Run `lastchange wallet migrate` (or just start `lastchange`, which offers it): it
encrypts the seed into the keystore with a new passphrase, then overwrites and deletes `seed.json`. Note: on flash
storage an overwrite is best effort; if the old file may have been copied, create a new wallet.
`wallet delete --yes` moves the folder to a backup and never keeps a plaintext seed there.

Interactive menu: run `lastchange` with no arguments (or `lastchange wallet`, or `scripts/menu.sh`). First pick a
language (English, Deutsch, Español, العربية, Français, 中文, Русский, Türkçe, فارسی; `--lang xx` skips it). Then:
1) My wallet 2) Manage wallet (new / restore / show / delete / journal) 3) Look up another address 4) Explorer
5) Send (info: not available, no coins) 6) Exit. `--json` gives JSON for the info screens. Word counts:
12 / 15 / 18 / 24. 16 is not a valid BIP39 length. 24 is recommended.

Paper journal: menu option 5 records PAPER buy/sell entries (asset, quantity, price, optional fee, note) in a local `paper_journal.db`. It shows FIFO positions, average cost, realized P&L, and unrealized P&L at a price you type in, and can export CSV. It is never sent to the ledger or the network and involves no real money.
