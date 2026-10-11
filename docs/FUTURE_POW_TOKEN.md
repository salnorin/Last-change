# EST: «اعتیادزدایی جامع» (Comprehensive Addiction Treatment), a future PoW token

> **STATUS: NOT IMPLEMENTED. DESIGN ONLY. DISABLED PHASE. (وضعیت: پیاده‌سازی نشده، فقط طراحی)**
> Live code contains **zero** token, mining, wallet, swap or payment logic. CI enforces this with
> `scripts/check_no_token.sh`. Nothing in this document may be implemented before a German legal
> opinion is obtained (see §12). The marketplace works fully **without** EST.

## 1. Summary (خلاصه)
- Name / نام: **اعتیادزدایی جامع**. Ticker: **EST**.
- **Optional:** it would never be required to post, browse, rate or trade on Last Change.
- Users mine with their own CPU, and rewards go to **their own** wallet. There is no premine and no foundation share.
- **Pure P2P:** no exchange, no middleman, no fiat on/off-ramp in the design. EST would be used only to buy and sell inside the marketplace.
- Anyone holding mined EST could take part with **no profile or ID**, only pseudonymous keys (کلید مستعار).
- **Separate from anti-spam:** the live micro-PoW stamp (`DEFAULT_STAMP_BITS = 16`) protects against spam,
  creates no value, has no chain and is **not mining** (استخراج نیست). The two must never be merged.

## 2. Why P2P (مزیت همتابه‌همتا)
There is no third party, exchange or custodian. The market itself is the mechanism: listings, reputation
and settlement are all peer-to-peer, so nothing exists that could freeze, censor or misuse funds. The
cost of that is that there is no recourse. The reputation system (receipt-gated ratings) stays the
main protection against fraud.

## 3. Hash function choice (تابع هش)
Use a memory-hard function (حافظه‌بر) for CPU fairness: **RandomX** (proven on Monero) or **Argon2id**
(simpler, but GPU-friendlier at low memory). The baseline is RandomX-like (see §5) with BLAKE3/SHA3 for
everything else.

## 4. EST-PoW: multi-hash + circuit blocks
### 4a. Multi-hash chain (زنجیره‌ی چندهشی، شبیه X11)
Candidate functions: SHA-256, SHA-512, SHA3-256, BLAKE3, Keccak-256, Skein-512, Groestl-512, JH, BLAKE2b.
For each block the **order is permuted** deterministically from `prev` (Fisher–Yates seeded by `BLAKE3(prev)`).

**Honest note:** chaining doesn't add up the strength of the functions. Collision/preimage security is
roughly that of the **weakest link**. The benefit is ASIC-resistance and diversity (an ASIC must
implement all of them, in any order), not stronger security.

### 4b. Circuit blocks (بلوک‌های مداری)
The `circuit_seed` of each header deterministically generates a small random program, a RandomX-like VM
with integer and float ops, conditional branches and a memory-hard scratchpad (for example 256 MiB for
miners and a light 2 MiB cache mode for verifiers).
Mining loop: `nonce → circuit(seed, nonce) → output → multi_hash(order(prev), output) < target`.

- **Verification cost:** one circuit execution plus one multi-hash, in milliseconds. It must stay at least
  10³× cheaper than mining.
- **Determinism across CPUs:** floats use IEEE-754 with a fixed rounding mode, with no FMA contraction and no
  transcendental functions. Integer ops use wrapping semantics. Test vectors run in CI on x86_64 and aarch64.
- **Bounds:** 256–2048 gates, branch depth ≤ 8, loops bounded by a fixed iteration counter, so every
  circuit terminates.

```rust
pub struct BlockHeader {
    pub version: u32,
    pub prev: [u8; 32],
    pub merkle_root: [u8; 32],   // BLAKE3 Merkle tree of transactions
    pub time: u64,
    pub target: [u8; 32],
    pub nonce: u64,
    pub circuit_seed: [u8; 32],  // = BLAKE3(prev || height)
    pub pq_sig: Vec<u8>,          // optional miner ML-DSA signature
}
pub enum Gate {
    IAdd(u8, u8, u8), IMul(u8, u8, u8), IXor(u8, u8, u8), IRot(u8, u8, u32),
    FAdd(u8, u8, u8), FMul(u8, u8, u8), FDiv(u8, u8, u8), FSqrt(u8, u8),
    Load { dst: u8, addr_reg: u8 }, Store { src: u8, addr_reg: u8 },
    BranchIfZero { reg: u8, mask: u64, skip: u16 },
}
pub struct Circuit { pub gates: Vec<Gate>, pub iterations: u32, pub scratchpad_bytes: usize }
```

### 4c. Difficulty: LWMA (تنظیم سختی)
A per-block Linearly Weighted Moving Average over N = 60 blocks, with a target block time of 120 s, solve
times clamped to [−6T, 6T] and timestamps checked against the median of the last 11 blocks.

### 4d. Optional "useful work" variant (کار مفید)
The idea is to replace the circuit with useful computation (for example ML training or protein folding).
The problems: results are hard to verify cheaply and deterministically, the work can be precomputed or
outsourced, and hardness is uneven. Because of that it is **not recommended**, and at most a research note.

## 5. Supply schedule (برنامه‌ی عرضه)
The initial reward is 50 EST per block, halving (نصف‌شدن) every 525,600 blocks (about 2 years at 120 s),
with a hard cap of about 52.56 M EST. A small tail emission is optional, as a later consensus choice.

## 6. Ledger structure (ساختار دفتر)
- **UTXO model** like Bitcoin. Blocks contain coinbase + transactions, committed with a BLAKE3 Merkle root.
- **Full-node wallet** in Rust, like Bitcoin Core: it runs its own node, validates everything and has HD keys (BIP32-style
  derivation adapted to PQ keys through seed-based deterministic keygen). Keys never leave the device.

## 7. Post-quantum crypto (رمزنگاری پساکوانتومی)
- Signatures: **ML-DSA (Dilithium, FIPS 204)** as the default, and **SLH-DSA (SPHINCS+, FIPS 205)** as a conservative option.
- P2P key exchange: **ML-KEM (FIPS 203)**, hybrid with X25519.
- Hashing: SHA3 and BLAKE3.
- **Trade-offs:** an ML-DSA-65 signature is about 3.3 KB (about 2.4 KB at ML-DSA-44), compared with 64 B for ed25519. SLH-DSA is
  8–30 KB. A 1-in/2-out transaction grows about 40× compared with ECDSA, and so does the ledger (about 10 GB/year at
  moderate load). Verification is still fast for ML-DSA and slower for SLH-DSA. Mitigations: signature
  aggregation research, pruning, and snapshots.

## 8. How mining could gate purchases (اتصال به بازار)
A listing could carry an optional `accepts_est: true` flag. The buyer pays the seller directly with an EST
transaction, and the seller's DealReceipt references the txid. The marketplace never holds funds, never
matches orders and never runs escrow.

## 9. Multi-asset listings: XMR and delisted coins (چنددارایی)
- A listing could say "accepts XMR / BTC / EST". The wallet would talk to the **user's own** `monerod` /
  `monero-wallet-rpc` or Bitcoin node, with no custody.
- Instead of escrow, **trustless atomic swaps** XMR↔BTC/EST using adaptor signatures (COMIT xmr-btc-swap,
  Farcaster-style). Either both legs complete or both refund.
- The marketplace never holds funds or keys and runs no swap service.

## 10. Attacks (حملات)
- **51%:** a young, small chain is cheap to attack. Mitigations: deep confirmations for trades and checkpoints
  (with a centralization trade-off). Merged mining is possible, but it increases complexity.
- **Selfish mining:** a uniform-random tie-break, and propagation of headers before the full block.
- Timestamp manipulation (LWMA clamps), eclipse attacks (diverse peers, anchor connections), dust spam (minimum fee).

## 11. Energy cost (هزینه‌ی انرژی)
CPU mining by participants still burns electricity, with no value outside the chain. In Germany, electricity
at about €0.30/kWh likely makes mining unprofitable, so it would be a hobby. That has to be stated honestly to
users.

## 12. Legal (حقوقی): read before anything else
- EST may be a crypto-asset under **MiCA**. Even without an issuer company, premine or exchange, a mined
  payment token and the software around it **may still require legal analysis**: whether there is an
  issuer/offeror, whether running nodes, wallets or the market counts as a **CASP** service (custody, exchange,
  transfer), and **KWG** licensing (crypto custody / financial services) with **BaFin**.
- **Transfer of Funds Regulation (EU 2023/1113)**: the travel rule applies to CASPs.
- **EU AML Regulation (2024/1624)**: from **July 2027**, CASPs may not handle anonymity-enhancing coins
  (for example XMR) or anonymous accounts. Running a market or swap service with such features may be a
  regulated, or prohibited, crypto service in Germany/EU.
- **Obtain German counsel (Fachanwalt Bank-/Kapitalmarktrecht) before any implementation.**
- Status: **NOT implemented.** Live code is checked by CI to contain none of this.

## Authors (نویسندگان)
- Creator: **Salar Khanbaba (salnorin)**
- Co-designer:
<!-- CO_DESIGNER -->
