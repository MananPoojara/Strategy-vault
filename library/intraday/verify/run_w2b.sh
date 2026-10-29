#!/bin/bash
# Start the test engine for ONE market on its private port, run compare_w2b.py, stop it. Call under the engine flock:
#   flock .../claude-engine-run.lock library/intraday/verify/run_w2b.sh us|in OUT_DIR [id-prefix]
set -u
MKT=$1; OUT=$2
BIN=${BIN:-/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/target-w2b/release/guru}
D=/home/server/.cache/strategy-vault-webapp/data
M1=$D/intraday
HERE=$(dirname "$(readlink -f "$0")")
mkdir -p "$OUT"
if [ "$MKT" = us ]; then PORT=19945; ARGS=("$D/us_data.csv" $PORT "$D/us_vix" "$D/extra_data/us" "$M1"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/us-market-data
else PORT=19946; ARGS=("$D/idx_data.csv" $PORT "$D/india_vix" "$D/extra_data/india" "$M1"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/market-data-repaired-20261015-v2; fi
nice -n 10 "$BIN" "${ARGS[@]}" > "$OUT/server-$MKT.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null; wait $PID 2>/dev/null' EXIT
for i in $(seq 1 120); do curl -sf "http://127.0.0.1:$PORT/api/guru/connors/strategies" -o /dev/null && break; sleep 2; done
nice -n 10 ionice -c3 python3 "$HERE/compare_w2b.py" $PORT "$MKT" "$OUT/compare-w2b-$MKT${3:+-$3}.json" ${3:-} > "$OUT/compare-w2b-$MKT${3:+-$3}.txt" 2>&1
tail -1 "$OUT/compare-w2b-$MKT${3:+-$3}.txt"
