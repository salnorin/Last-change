# Security model: Bitcoin Core vs. lastchange

Honest status as of branch `i18n-menu`. "done" means implemented and tested here; it does **not**
mean audited. lastchange has had no external security review.

| Bitcoin Core feature | lastchange equivalent | Status | Where |
|---|---|---|---|
| Encrypted wallet (`encryptwallet`, AES-256-CBC, SHA-512 key stretching) | Keystore always encrypted: Argon2id (64 MiB, t=3, p=1) + XChaCha20-Poly1305; plaintext seed is never written | done | `crates/core/src/keystore.rs`, `crates/node/src/main.rs` (`save_wallet`, `wallet_new`) |
| Passphrase required to unlock | Mandatory passphrase, asked twice for new keystores, minimum-strength check (>=12 chars, 2+ classes or 20+ chars) | done | `main.rs` (`passphrase`, `passphrase_ok`) |
| `walletpassphrase <timeout>` unlock timeout | No unlock session at all: key decrypted per command and dropped (zeroized) right after; nothing cached | done (stricter, less convenient) | `main.rs` (`load_id`) |
| Keys wiped from memory | `zeroize`: master seed on `Identity` drop, derived sub-seeds, Argon2 key, decrypted plaintext, passphrase strings | partial (best effort: no `mlock`, copies inside dependencies / the OS can remain) | `crates/core/src/lib.rs`, `keystore.rs`, `main.rs` |
| BIP39 checksum (Core itself uses descriptors, not BIP39) | `bip39` crate verifies wordlist + checksum on restore; test covers a bad checksum | done | `lib.rs` (`from_mnemonic`, `bip39_tests`) |
| HD derivation (BIP32/44, secp256k1) | BIP32/44 do not apply to Ed25519. SLIP-10 Ed25519 hardened derivation implemented with official test vectors; **not yet used** for identities (switching changes every pseudonym, needs a versioned migration). Current: HKDF-SHA256 domain-separated sub-keys from one master; ML-DSA-65 keys deterministic from seed (`from_seed`) | partial | `crates/core/src/hd.rs`, `lib.rs` (`derive`, `from_master`) |
| Legacy plaintext wallet upgrade | `wallet migrate` / startup offer: encrypt `seed.json`, verify, overwrite+delete (best effort on flash) | done | `main.rs` (`wallet_migrate`, `shred`) |
| Full block/tx validation | Blocks: PoW, signature, zero-value coinbase rules validated on load. No transactions exist (no value by design) | partial | `crates/chain/src/lib.rs` (`validate`) |
| Consensus rules | Small, young rule set (zero-value testnet chain + signed ledger envelopes with anti-spam stamps). No fork-choice battle history | partial | `crates/chain`, `crates/ledger` |
| Peer misbehavior banning (DoS scores, `setban`) | gossipsub strict validation + Noise-encrypted transport; local `block` of pseudonyms/entries. **No automatic peer scoring/banning yet** | missing | `crates/net/src/lib.rs`, `main.rs` (`Cmd::Block`) |
| Reproducible builds (Guix) | `Cargo.lock` + `--locked` in CI/release. Not bit-for-bit reproducible, no signed attestations | partial | `.github/workflows/*.yml` |
| Constant-time secret comparisons | `subtle`-based `ct_eq` for passphrase repeat and delete confirmation; AEAD tag check is constant-time inside the crate | done | `lib.rs` (`ct_eq`), `menu.rs` |

## Crate evaluation

| Crate | Verdict |
|---|---|
| `bip39` (rust-bitcoin org) | **In use.** Pure Rust, builds on Termux. Provides checksum validation. |
| `rust-bitcoin` | Not used: secp256k1 + Bitcoin script/tx types; we have no Bitcoin transactions, and the no-value guard forbids it. |
| BDK | Not used: Bitcoin wallet (UTXO, balances, chain sync). Out of scope by design and blocked by `check_no_token.sh`. |
| `slip10` / `ed25519-hd-key` crates | Not added: small, lightly maintained. SLIP-10 is ~30 lines over `hmac`+`sha2` (already dependencies), so we implemented it in `hd.rs` and pinned it with the SLIP-0010 test vectors. |
| `zeroize`, `subtle`, `hmac` | **Added** (already in the lock file via dalek/RustCrypto; pure Rust; Termux-safe). |

## Beyond Bitcoin Core (honestly)

Things lastchange does that Core does not, with caveats:

- **Memory-hard KDF.** Argon2id instead of iterated SHA-512 makes GPU/ASIC passphrase guessing much more
  expensive. It does not help against a weak passphrase that is guessed quickly anyway.
- **Mandatory encryption + strength check.** Core allows unencrypted wallets; we refuse. The strength check is a
  heuristic, not a guarantee.
- **No cached unlocked key.** Every key use asks again. Safer if the device is left open, less convenient.
- **Post-quantum hybrid signatures** (Ed25519 + ML-DSA-65) and hybrid encryption (X25519 + ML-KEM-768).
  The ML-DSA/ML-KEM crates are young and unaudited.

Optional / future (not implemented):

- **Termux fingerprint gate**: call `termux-fingerprint` (Termux:API) before revealing words or the master secret.
  Convenience layer only; it does not replace the passphrase and can be bypassed by anyone with shell access.
- **Separate-process signing**: a small signer process holding the decrypted key, talking over a local socket,
  so the network/API process never sees secrets (similar in spirit to hardware or external signers).
- `mlock`/no-swap for secret pages, versioned SLIP-10 identity migration, peer scoring/banning, reproducible
  build attestations, and an external audit.

## What Bitcoin Core has that we do not

Bitcoin Core's real strength is not any single feature above: it is **more than a decade of adversarial
operation, battle-tested consensus code, extensive fuzzing, and years of review by many experts**. lastchange has
none of that yet. Treat it as experimental testnet software and never rely on it to protect value.
