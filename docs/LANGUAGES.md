# Languages used in Last-change

| Language | Where | Role |
|---|---|---|
| Rust | `crates/` | All core logic: keys, wallet, ledger, chain, reputation, P2P, API, menu (9 UI languages in `crates/node/src/i18n.rs`). |
| Shell | `scripts/` | Glue only: `menu.sh` (Termux/Linux launcher), CI guards, e2e. |
| Go | `tools/explorer-go` (optional) | Read-only explorer page on top of the Rust JSON API. Own `go.mod`, not part of the cargo build, never touches keys. |
| Assembly | none (by design) | See below. |

## Why no hand-written assembly

The crypto crates we depend on (ed25519-dalek / curve25519-dalek, ML-DSA / ML-KEM
implementations, SHA-2, ChaCha/AES backends) already ship audited, optimized
backends with runtime CPU-feature detection (SIMD on x86_64 and aarch64) and
portable fallbacks. Custom assembly would:

- **Risk security**: constant-time behavior is easy to break by hand, opening
  timing side channels; new code would be unaudited.
- **Hurt portability**: separate code per CPU (x86_64, aarch64 Android/Termux,
  armv7, Apple Silicon), harder cross-compilation, and more CI targets.
- **Gain little**: the hot paths are already vectorized; our workload (signing a
  few messages, deriving keys) is not CPU-bound.

If profiling ever shows a real bottleneck, the right step is enabling an
existing backend feature in the upstream crate, not writing our own asm.
