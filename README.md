# Last Change Marketplace

A peer-to-peer, pseudonymous classifieds marketplace with reputation, written in Rust.
People post **offers** (supply) and **requests** (demand), peers see them over a P2P
network, and buyers rate sellers after a deal. There is no central server and no account.

> ⚠️ **Legal notice / BaFin & MiCA.** This software has **no token, no cryptocurrency,
> no payments, no escrow and never holds anyone's funds.** Deals and payment are arranged
> between people outside the software. Adding a token, mining rewards, custody or a swap
> service could make operating it a regulated activity in Germany/EU (KWG, MiCA, AML rules).
> Any such idea lives only in [`docs/FUTURE_POW_TOKEN.md`](docs/FUTURE_POW_TOKEN.md) as a
> disabled, design-only concept, and requires German legal counsel before any implementation.
> CI enforces this with [`scripts/check_no_token.sh`](scripts/check_no_token.sh).

> **Anonymity does not exempt anyone from the law.** Illegal goods and services are forbidden.
> See [TERMS.md](TERMS.md).

---

## Manifesto

We believe that buying and selling is a natural human right, one that has existed since the
dawn of civilization and must never be taken away by monopolies or corruption.

Last Change restores this right with an intermediary-free marketplace that uses cryptography
and distributed networks instead of platforms, accounts and profiles.

## Features (implemented)

- **Pseudonymous identity:** an ed25519 keypair generated on your device. Your pseudonym is a hash
  of the public key. No name, email, phone, KYC or profile exists anywhere.
- **Offers and requests:** title, description, free-text price hint, category, expiry, image hash links
  (images stay off-ledger).
- **Signed envelopes:** every post is signed and carries a small proof-of-work **anti-spam stamp**
  (about 65k hashes, under a second). This is not mining and produces nothing of value.
- **Light ledger:** an append-only, signed, Merkle-committed log in SQLite on each node.
- **Reputation:** a buyer can rate only after the **seller signs a "deal completed" receipt** naming them.
  Ratings are weighted by the rater's own reputation and decay over time (half-life of 180 days).
- **P2P network:** libp2p with Noise encryption, gossipsub for propagation and Kademlia for discovery.
  The transport key is ephemeral and kept separate from your marketplace identity.
- **Safety:** a local content filter, local blocklists (pseudonym, key or entry) and gossiped report flags.

## Workspace

| crate | purpose |
|---|---|
| `crates/core` | identities, Listing / DealReceipt / Rating / Report, signed envelopes, stamp, content filter |
| `crates/ledger` | SQLite append-only log, Merkle root, blocklist |
| `crates/reputation` | receipt-gated, weighted, decaying scores |
| `crates/net` | libp2p swarm (gossipsub, kademlia, noise, yamux) |
| `crates/node` | `lastchange` CLI |

## Usage

```sh
cargo build --release
B=target/release/lastchange
$B keygen
$B post offer --title "Used bike" --price-hint "negotiable" --category bikes
$B post request --title "Looking for a desk lamp"
$B list            # or: $B search bike
$B peers --listen /ip4/0.0.0.0/tcp/4001 --dial /ip4/1.2.3.4/tcp/4001/p2p/<peer-id>
# seller, after the deal happened off-network:
$B complete-deal <listing-id> --buyer <buyer-pubkey>
# buyer:
$B rate <receipt-id> --score 5 --comment "as described"
$B report <entry-id> --reason "illegal item"; $B block <pseudonym>
```

Run the tests with `cargo test --workspace`. They include a two-node libp2p integration test.

## Phases

1. **Core (this release):** the CLI, ledger, reputation and P2P sync over TCP.
2. **Usability:** NAT traversal (relay, DCUtR), bootstrap peers, an image store over content hashes, a desktop/Android UI.
3. **Privacy transport (docs only for now):** optional Tor or I2P transport, so IP addresses aren't linkable to pseudonyms.
4. **Hardening:** rate limits per peer, filter lists shared and signed by communities, ledger pruning and snapshots.
5. **Future, disabled:** the token concept in `docs/FUTURE_POW_TOKEN.md`. It is **not implemented** and is blocked by CI and by legal review.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). PRs that add token, payment, custody or mining logic to live code will fail CI.
