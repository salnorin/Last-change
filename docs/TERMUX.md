# Using the wallet CLI on Android (Termux)

Phones only hold keys and sign. They **never mine and never compute heavy stamps or proofs**: `lastchange mine` refuses to run on Android, and posting from a phone should go through your own desktop/server node (`POST /api/stamp`).

## Option A: download a binary
1. Open the latest **release** workflow run (GitHub → Actions → `release`) and download `lastchange-wallet-aarch64-linux-android`.
2. In Termux:
   ```sh
   termux-setup-storage
   cp ~/storage/downloads/lastchange-wallet-aarch64-linux-android ~/lastchange
   chmod +x ~/lastchange
   ~/lastchange wallet new
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
