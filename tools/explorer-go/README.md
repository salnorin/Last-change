# explorer-go (optional)

Read-only Go front-end for the Rust explorer API. Stdlib only, no keys, no ledger writes.

```
lastchange explorer &            # Rust API on 127.0.0.1:8090 (full build)
cd tools/explorer-go && go run . # page on http://127.0.0.1:8091
```

Termux: `pkg install golang` then the same commands. Note the wallet-only
Termux build of `lastchange` has no `explorer` server, so point `-api` at a
desktop node, e.g. `go run . -api http://192.168.1.10:8090`.
