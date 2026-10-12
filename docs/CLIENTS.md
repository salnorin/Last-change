# Clients: web, Android, web3 (status and plan)

## Rule: light clients never do proof-of-work or heavy proofs
Mobile and web clients **never** run mining, circuit-pow stamps or STARK proving. Heavy work is done by the
user's own desktop/server `lastchange serve` node, or by a node the user trusts:
- **Today (implemented):** the web app calls the local node API, and the node signs and stamps.
- **Implemented (envelope v2):** the client signs with its own hybrid key (nonce = 0) and `POST /api/stamp`. The node verifies,
  computes the stamp (bound to the signatures, outside them) and publishes. `POST /api/envelope` accepts fully sealed envelopes.

## Web app (`web/`, served by the node at `/`)
- Implemented: a mobile-first single page with a listing list, search and filter, a create-ad form (offer/request,
  art category, image hashes), Proof of Trust buttons (accept, paid, shipped, confirm, dispute, claim), deal
  receipt and rating, and live updates over WebSocket.
- Planned: key-based "login", where a key is generated or imported locally, encrypted in the browser
  (WebCrypto AES-GCM with a PBKDF2/Argon2 passphrase) and stored in IndexedDB. There are no profiles.

## Android (planned skeleton)
A Kotlin (Jetpack Compose) UI shell with the same screens, consumer-only. A Rust core (`lc-core` types,
signing, verification) is exposed through **uniffi**, and the app talks to the user's node over HTTPS/WebSocket.
Planned layout: `android/app` (Kotlin) and `crates/ffi` (uniffi bindings), with a CI job building a debug APK with
cargo-ndk.

## web3 frontend (planned skeleton)
Signs with the user's own ed25519 key (WebCrypto). The "connect wallet" button stays **disabled**, as a
design-only placeholder. No token or payment logic exists (see `docs/FUTURE_POW_TOKEN.md`).

## API (node, default 127.0.0.1:8080)
`GET /api/whoami | /api/listings?q=&kind=&category= | /api/entries | /api/trust/:pubkey`,
`POST /api/post | accept | step | claim | complete | rate | report | envelope`, `GET /ws` (new entries).
