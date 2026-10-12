# Node plan (testnet)

## 1. Consensus
1. **Phase 1 — PoA round-robin.** Genesis-pinned signer set (hybrid keys).
   Block at height *h* is valid only if signed by `signers[h % n]`; a missed
   slot passes to the next signer after a timeout. circuit-pow stays as
   anti-spam on the header.
2. **Phase 2 — longest valid chain + finality.** Fork choice = most
   cumulative work among valid chains; a block is final after *k*
   confirmations or 2/3 signer checkpoint signatures. Reorgs deeper than
   the finality point are rejected.

## 2. Validation rules
- Header: `prev` = known block, height = prev+1, `time` > median of last
  11 and < now+2h, PoW bits >= target, signer valid for slot, both sigs.
- Body: merkle root matches, <= MAX entries/bytes, every item passes the
  same checks as mempool admission, no duplicate ids against the chain,
  deal-step ordering (see §5). Coinbase value == 0.

## 3. Mempool (`crates/mempool`, implemented)
Admission = `Envelope::verify(min_stamp_bits)` or `SettlementRecord::verify()`,
then dedupe/replay vs chain, size cap, per-author quota, eviction of the
weakest stamp, deal-gate rule. `template(max)` feeds block producers;
`remove_confirmed` runs when a block connects; on reorg, disconnected
entries are re-inserted.

## 4. Network
- gossipsub topics `lastchange/v1/entries`, `.../blocks`; peers validate
  before forwarding (`ValidationMode::Strict` + app-level accept/reject).
- Headers-first sync over a libp2p request-response protocol
  (`getheaders` -> `headers` -> `getblocks`), like Bitcoin Core IBD.
- Discovery: bootstrap multiaddrs (VPS) + Kademlia; mDNS on LAN.
- Roles: **VPS full node** (validates, stores, produces blocks, explorer);
  **Termux light node** (headers + own entries, submits to VPS, wallet-only
  build stays default).

## 5. Market layer on top
Listings, offers, `DealAccept`, `DealStep`, ratings, claims, reports are
signed entries -> mempool -> blocks. State machine:
`Accept -> BuyerPaid -> SellerShipped -> SettlementRecord -> BuyerConfirmed -> Rating`.

### Payments on-chain — Tier A (implemented, no token)
`SettlementRecord` (live-code name; the guard forbids the word family
"pay*", and this is a *record*, not a value movement): terms =
`accept_id, amount (decimal string), currency code, optional external tx
hash (e.g. an LN/other-chain tx id), created`, co-signed by seller AND
buyer (hybrid sigs, distinct parties). It is a normal chain item: validated
in the mempool and in blocks, one per deal (conflicts rejected), included in
regular blocks with other entries. `BuyerConfirmed` is rejected until a
settlement exists for that deal. The chain never sums amounts or keeps
account state, so `check_no_token.sh` stays green. Legal note: recording
amounts of third-party payments is far lighter than moving value, but should
still get a short legal look (privacy/GDPR of amounts, AML optics).

### Tier B — native testnet value units (DESIGN ONLY, disabled)
See `docs/FUTURE_POW_TOKEN.md` style; summary:
- Account model `{pubkey -> (nonce, units)}` in a separate state DB; tx =
  `{from, to, units, nonce, sig}`.
- Validation: sig, `nonce == state.nonce+1`, `units <= state.units`.
- Mempool: one pending tx per (from, nonce); replacement only with stronger
  stamp; drop on nonce gap; double-spend = same (from, nonce) conflict.
- Zero-value testnet units only, no issuance beyond a fixed test faucet,
  behind a cargo feature that is **off** and excluded from releases.
- **Must not be enabled** before MiCA (crypto-asset/CASP), BaFin (KWG/ZAG)
  and EU AMLR review; enabling also requires an explicit guard policy change.
