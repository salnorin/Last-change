#!/usr/bin/env bash
# Guard for live code (crates/, Cargo.toml, Cargo.lock). docs/ is design-only and excluded.
#
# Policy (updated when the owner approved a zero-value TESTNET miner):
#   ALLOWED:   mining / miner / proof-of-work / testnet blocks / coinbase with value 0,
#              identity keystore ("wallet" = keys only).
#   FORBIDDEN: balances, rewards, minting, value transfers, payments, swaps,
#              exchanges, escrow, custody, UTXO value accounting, tokens.
# The miner crate must additionally assert coinbase value == 0 in its tests.
# Comments are ignored.
set -euo pipefail
cd "$(dirname "$0")/.."
PAT='\b(token|tokens|balance|balances|get_balance|reward|rewards|block_reward|mint|minting|premine|transfer|transfers|transfer_funds|payment|payments|pay|swap|swaps|atomic_swap|exchange|exchanges|escrow|custody|utxo|utxos|monero|xmr|bitcoin|btc)\b'
# strip // comments (whole-line and trailing) and the slice method `.swap(` before matching
hits=$(grep -rniE "$PAT" --include='*.rs' --include='Cargo.toml' crates Cargo.toml \
  | sed -E 's#//.*$##; s#\.swap\(##g' | grep -iE "^[^:]+:[0-9]+:.*$PAT" || true)
deps=$(grep -E '^name = "' Cargo.lock | grep -iE '"(bitcoin|monero|ethers|web3|alloy|solana|bdk|secp256k1-zkp)' || true)
if [[ -n "$hits$deps" ]]; then echo "Forbidden value/payment logic found:"; echo "$hits"; echo "$deps"; exit 1; fi
echo "OK: no balances, rewards, minting, transfers, payments, swaps, exchange, escrow or custody in live code."
