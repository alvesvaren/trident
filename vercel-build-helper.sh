#!/bin/bash
set -euo pipefail

source_rust_env() {
  local candidates=(
    "${CARGO_HOME:-}/env"
    "$HOME/.cargo/env"
    "/rust/env"
  )

  for env_file in "${candidates[@]}"; do
    if [ -n "$env_file" ] && [ -f "$env_file" ]; then
      # shellcheck source=/dev/null
      source "$env_file"
      return 0
    fi
  done

  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/rust/bin:${PATH}"
}

if ! command -v rustc >/dev/null 2>&1; then
  echo "Installing Rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi

source_rust_env

echo "Ensuring wasm32 target is installed..."
rustup target add wasm32-unknown-unknown

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "Installing wasm-pack..."
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh -s -- -y
fi

echo "Building app..."
pnpm build
