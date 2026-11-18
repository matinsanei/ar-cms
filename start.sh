#!/bin/bash
set -e

echo "=========================================================="
echo "  🚀 Starting Matin CMS (Strapi Admin + Rust Unified Runtime)"
echo "=========================================================="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 1. Start Rust Engine
echo "[1/2] Launching Rust CMS Engine on port 8088..."
cd "$SCRIPT_DIR/server/cms-engine"
./target/release/cms-engine &
RUST_PID=$!

trap "kill $RUST_PID 2>/dev/null || true" EXIT

sleep 1

# 2. Start Official Strapi Admin Panel via Vite
echo "[2/2] Launching Official Strapi Admin Panel on http://localhost:1337..."
cd "$SCRIPT_DIR"
pnpm dev --port 1337 --host 0.0.0.0
