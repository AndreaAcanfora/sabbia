#!/usr/bin/env bash
# Builds the WebAssembly module into web/, ready to serve: python3 -m http.server -d web
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/sabbia.wasm web/sabbia.wasm
echo "web/sabbia.wasm: $(wc -c < web/sabbia.wasm) bytes"
