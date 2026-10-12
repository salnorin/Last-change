# Using the wallet CLI on Android (Termux)

Phones only hold keys and sign. They **never mine and never compute heavy stamps or proofs**: `lastchange mine` refuses to run on Android, and posting from a phone should go through your own desktop/server node (`POST /api/stamp`).

## Option A: download a binary
1. Open the latest **release** workflow run (GitHub → Actions → `release`) and download `lastchange-wallet-aarch64-linux-android`.
2. In Termux:
   ```sh
   termux-setup-storage
   cp ~/storage/downloads/lastchange-wallet-aarch64-linux-android ~/lastchange
   chmod +x ~/lastchange
   ~/lastchange wallet new --words 24
   ~/lastchange wallet show
   ```

## Option B: build in Termux
```sh
pkg install rust git clang
git clone https://github.com/salnorin/Last-change.git
cd Last-change
cargo build --release -p lastchange --no-default-features --features wallet
./target/release/lastchange wallet new
```
SQLite is bundled (`rusqlite` `bundled` feature) and there is no OpenSSL dependency, so no extra system libraries are needed.

## Wallet-only build
The `wallet` feature leaves out libp2p, the HTTP server and the explorer, so there's no `ring`, no OpenSSL and a fast build:
`cargo build --release -p lastchange --no-default-features --features wallet`.
It contains: `wallet new|show|export`, `msg-seal|msg-open`, and local ledger commands.

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
