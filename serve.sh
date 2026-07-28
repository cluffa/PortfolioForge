#!/bin/sh
# Serve the PortfolioForge web app locally
# Requires: wasm-pack, Python 3
set -e

echo "Building WASM..."
wasm-pack build crates/portfolio-wasm --target web --out-dir ../../apps/web/public/wasm

echo ""
echo "Starting server at http://localhost:8080"
cd apps/web && python3 -m http.server 8080
