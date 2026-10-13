#!/usr/bin/env bash
# Start the Strategy Vault for the whole LAN: Rust API on :8090, UI on :5173.
# Ctrl+C stops both.
set -e
cd "$(dirname "$0")"
(cd backend && cargo build --release -q)
BIN="${CARGO_TARGET_DIR:-backend/target}/release/guru"
"$BIN" &
API=$!
trap 'kill $API 2>/dev/null' EXIT
IP=$(hostname -I | awk '{print $1}')
echo "Open on any PC in the LAN: http://$IP:5173"
cd frontend && python3 -m http.server 5173 --bind 0.0.0.0
