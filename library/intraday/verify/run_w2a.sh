#!/bin/bash
# Wave 2a: start ONE market's test engine on its private port, run a command against it, stop it. Call under the engine flock:
#   flock .../claude-engine-run.lock library/intraday/verify/run_w2a.sh BIN us|in LOG -- command args...   ($PORT is exported)
set -u
BIN=$1; MKT=$2; LOG=$3; shift 4
D=/home/server/.cache/strategy-vault-webapp/data
M1=$D/intraday
if [ "$MKT" = us ]; then PORT=19935; ARGS=("$D/us_data.csv" $PORT "$D/us_vix" "$D/extra_data/us" "$M1"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/us-market-data
else PORT=19936; ARGS=("$D/idx_data.csv" $PORT "$D/india_vix" "$D/extra_data/india" "$M1"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/market-data-repaired-20261015-v2; fi
export PORT
nice -n 10 "$BIN" "${ARGS[@]}" > "$LOG" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null; wait $PID 2>/dev/null' EXIT
for i in $(seq 1 150); do curl -sf "http://127.0.0.1:$PORT/api/guru/connors/strategies" -o /dev/null && break; sleep 2; done
nice -n 10 ionice -c3 "$@"
