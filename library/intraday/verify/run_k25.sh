#!/bin/bash
# K25 (ss_momentum_pinball, tdperl_rebo): start the test engine for ONE market on its private port, run compare_k25.py, stop it (by PID).
# Call through the heavy wrapper and the engine flock:
#   authors/heavy flock .../claude-engine-run.lock library/intraday/verify/run_w3.sh us|in OUT_DIR [id-prefix]
set -u
MKT=$1; OUT=$2
BIN=${BIN:-/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/target-k2/release/guru}
D=/home/server/.cache/strategy-vault-webapp/data
HERE=$(dirname "$(readlink -f "$0")")
mkdir -p "$OUT"
if [ "$MKT" = us ]; then PORT=19947; ARGS=("$D/us_data.csv" $PORT "$D/us_vix" "$D/extra_data/us" "$D/intraday"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/us-market-data
else PORT=19948; ARGS=("$D/idx_data.csv" $PORT "$D/india_vix" "$D/extra_data/india" "$D/intraday"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/market-data-repaired-20261029-v3; fi
nice -n 10 "$BIN" "${ARGS[@]}" > "$OUT/server-$MKT.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null; wait $PID 2>/dev/null' EXIT
for i in $(seq 1 300); do curl -sf "http://127.0.0.1:$PORT/api/guru/connors/strategies" -o /dev/null && break; sleep 2; done
M1_DIR=$D/intraday nice -n 10 ionice -c3 python3 "$HERE/compare_k25.py" $PORT "$MKT" "$OUT/compare-k25-$MKT${3:+-$3}.json" ${3:-} > "$OUT/compare-k25-$MKT${3:+-$3}.txt" 2>&1
tail -1 "$OUT/compare-k25-$MKT${3:+-$3}.txt"
