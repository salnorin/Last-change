#!/usr/bin/env sh
# Install the `lastchange` CLI from source (Linux/macOS/Termux).
set -eu
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust not found. Install it first: https://rustup.rs (Termux: pkg install rust)"; exit 1
fi
SRC="${LASTCHANGE_SRC:-$HOME/.lastchange-src}"
if [ ! -d "$SRC/.git" ]; then
  git clone --depth 1 https://github.com/salnorin/Last-change.git "$SRC"
else
  git -C "$SRC" pull --ff-only
fi
cargo install --locked --path "$SRC/crates/node"
echo "Installed: $(command -v lastchange)"
echo "Next: lastchange wallet new"
