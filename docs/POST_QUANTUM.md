# Post-quantum cryptography in Last-change

## What is implemented
| Purpose | Algorithm | Notes |
|---|---|---|
| Signatures on every envelope (listings, Proof-of-Trust steps, receipts, ratings, claims, reports) | **Hybrid Ed25519 + ML-DSA-65 (FIPS 204)** | **Both** signatures must verify. An attacker must break both. |
| Pseudonym | `sha256(ed25519_pub ‖ ml_dsa_pub)[..20]` | No other identity data |
| Encrypted deal messages | **Hybrid X25519 + ML-KEM-768 (FIPS 203)** → HKDF-SHA256 → ChaCha20-Poly1305 | Confidential as long as either primitive holds |
| Keystore | 32-byte master seed, **Argon2id** (64 MiB, t=3) + XChaCha20-Poly1305 | All keys derived from the seed with HKDF |
| Anti-spam stamp | circuit-pow (hash-based) | Bound to the signatures, but outside them |

Libraries: RustCrypto `ml-dsa`, `ml-kem`, `ed25519-dalek`, `x25519-dalek`, `argon2`, `chacha20poly1305`.

## Honest limits
- **The libp2p transport (Noise XX, X25519) is not post-quantum yet.** An attacker who records traffic today could
  later decrypt the *gossip transport*. Gossip is public data anyway (signed listings). Private deal
  details must go through the hybrid ML-KEM channel (`lastchange msg-seal / msg-open`), which is post-quantum.
- Deal messages are currently exchanged out of band (copy/paste JSON). Direct libp2p request/response delivery is planned.
- **SLH-DSA (SPHINCS+, FIPS 205)** is a more conservative, hash-only alternative with 8–30 KB signatures.
  It could be offered as a third signature option for high-value identities.

## Measured (release build, this box)
- ML-DSA-65: signature 3309 B, public key 1952 B (Ed25519: 64 B / 32 B)
- One envelope as JSON: about 10.8 KB (hex) compared with about 0.4 KB before. That means about 25× ledger growth per entry.
- Hybrid sign about 3.5 ms, hybrid verify about 0.5 ms
- ML-KEM-768: ciphertext 1088 B, encapsulation key 1184 B; seal about 0.17 ms, open about 0.24 ms
- Keystore unlock (Argon2id 64 MiB, t=3): about 130 ms

## Light clients
Phones and browsers sign (`Identity::sign`, nonce = 0) and send the envelope to `POST /api/stamp`. The node
verifies both signatures, computes the circuit-pow stamp, and stores and publishes it. The envelope id does not
depend on the stamp nonce.
