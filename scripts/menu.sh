#!/usr/bin/env sh
# Termux/Linux glue: build the wallet-only binary if missing, then open the menu.
# All logic lives in the Rust binary. Extra args (e.g. --lang de, --json) are passed through.
set -e
cd "$(dirname "$0")/.."
B=./target/release/lastchange
if [ ! -x "$B" ]; then
  cargo build --release -p lastchange --no-default-features --features wallet
fi
exec "$B" "$@"
