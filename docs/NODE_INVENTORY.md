# Node inventory: Bitcoin Core subsystems vs Rust ecosystem vs Last-change

Last-change ports Bitcoin Core's *node architecture* (validation, mempool,
P2P, sync, reorg, storage, RPC), **not** its coins. There is no UTXO value,
no balances, no rewards (see `scripts/check_no_token.sh`, MiCA/BaFin).

Status: **done** / **partial** / **missing** (as of branch `node-core`).

| Bitcoin Core subsystem | Rust ecosystem | Last-change today | Status |
|---|---|---|---|
| Consensus / block validation (`validation.cpp`, `CheckBlock`, `ContextualCheckBlock`) | rust-bitcoin: block/header types + PoW check, no full rules; `bitcoinkernel` bindings wrap Core's C++; Floresta: full Utreexo validation | `crates/chain`: `Header`, `Block::validate()` (PoW via circuit-pow, hybrid ed25519+ML-DSA sig, zero coinbase), hardcoded genesis | partial (single block; no prev-link/timestamp/height/difficulty rules) |
| Tx validation (`CheckTransaction`, script interpreter) | rust-bitcoin script types; `bitcoinconsensus` crate (C++ libconsensus) | `lc_core::Envelope::verify` (hybrid sigs, payload rules, content filter, anti-spam stamp) | done for entries (no scripts — not needed) |
| Mempool (`txmempool.cpp`, policy, eviction, RBF) | none standalone; Floresta/Kyoto do not keep a full mempool | **new** `crates/mempool`: admission = full verify, dedupe/replay, size cap, per-author quota, weakest-stamp eviction, block template, co-signed `SettlementRecord` | partial (no persistence, no expiry yet) |
| P2P: version/addr/inv/getdata | rust-bitcoin `p2p` message types; nakamoto/Kyoto (BIP157/158 light clients) | `crates/net`: libp2p TCP+Noise+Yamux, gossipsub topic, Kademlia, identify | partial (gossip only; no inv/getdata request-response) |
| Headers-first sync (`getheaders`/`headers`) | Kyoto, nakamoto, Floresta | — | missing |
| Chain reorg / best-chain selection (`ActivateBestChain`) | Floresta | `LocalChain` holds tip=genesis only | missing |
| Mining (`getblocktemplate`, `miner.cpp`) | — (external miners) | circuit-pow solver, `mine_genesis`; mempool `template()` | partial (no block producer loop) |
| RPC / REST (`rpc/*.cpp`) | `bitcoincore-rpc` (client only) | `crates/node` CLI + `api.rs` HTTP + explorer, `--json` | partial |
| Wallet (`wallet/*`) | BDK, rust-bitcoin bip32 | BIP39 identity, hybrid keys, keystore (Argon2id, other worker on `i18n-menu`) | partial (identity only — by policy no coin wallet) |
| Storage (LevelDB block index, chainstate, `blk*.dat`) | Floresta (kv), sled/rocksdb | `ledger.db` (SQLite, merkle root), `chain.json` | partial (no block index) |
| Peer reputation / DoS ban score | — | `crates/reputation` (marketplace trust), ledger `block()` | partial (no network ban score) |

## Reuse decisions
- **rust-bitcoin / BDK / bitcoin_hashes**: not linked. They model Bitcoin
  transactions/UTXO/secp256k1, which Last-change does not use (keys are
  ed25519 + ML-DSA-65), and the guard rejects `bitcoin`/`bdk` crates. We copy
  *designs* (headers-first, inv/getdata, mempool policy), not code.
- **Floresta, Kyoto, nakamoto**: reference for sync state machines and
  light-client behaviour (Termux light node).
- **libp2p** (already used): replaces Bitcoin's hand-written P2P layer
  (encryption, peer ids, gossipsub, Kademlia, mDNS).
