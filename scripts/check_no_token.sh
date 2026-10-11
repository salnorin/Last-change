#!/usr/bin/env bash
# Fails if live code contains token / mining / payment / custody logic.
# Comment lines (//, //!, ///) are ignored; docs/ is design-only and excluded.
set -euo pipefail
cd "$(dirname "$0")/.."
PAT='\b(token|tokens|coin|coins|wallet|wallets|mining|miner|mine_block|block_reward|reward|rewards|utxo|escrow|payment|payments|pay|transfer_funds|balance|halving|monero|xmr|bitcoin|btc|est_token|atomic_swap)\b'
hits=$(grep -rniE "$PAT" --include='*.rs' --include='Cargo.toml' crates Cargo.toml | grep -vE '^[^:]+:[0-9]+:\s*//' || true)
deps=$(grep -E '^name = "' Cargo.lock | grep -iE '"(bitcoin|monero|ethers|web3|alloy|solana|randomx|rust-randomx|bdk|secp256k1-zkp)' || true)
if [[ -n "$hits$deps" ]]; then echo "Forbidden token/payment logic found:"; echo "$hits"; echo "$deps"; exit 1; fi
echo "OK: no token, mining, wallet or payment logic in live code (crates/, Cargo.toml, Cargo.lock)."
