#!/bin/bash
set -e

echo "Installing Rustup..."
# Install Rustup (compiler)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# Adding binaries to path (rustup installs into $CARGO_HOME, which Vercel sets to /rust)
. "${CARGO_HOME:-$HOME/.cargo}/env"
# wasm-pack doesn't detect rustup in Vercel's image, so add the wasm target explicitly
rustup target add wasm32-unknown-unknown

echo "Installing wasm-pack..."
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh -s -- -y

echo "Building app..."
# Build wasm
pnpm build
