#!/bin/bash
# Start the test engine for ONE market on its private port (with the staged option chains as arg 6), run compare_opt.py,
# stop it. Call under the engine flock:
#   flock .../claude-engine-run.lock library/options/verify/run_opt.sh us|in OUT_DIR [id filter | checks]
set -u
MKT=$1; OUT=$2
BIN=/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/targets/options-p2/release/guru
D=/home/server/.cache/strategy-vault-webapp/data
M1=/home/server/strategy-vault-data-staging/intraday-phase1-20261024/m1
ST=/home/server/strategy-vault-data-staging/options-phase2-20261024
HERE=$(dirname "$(readlink -f "$0")")
mkdir -p "$OUT"
if [ "$MKT" = us ]; then PORT=19945; ARGS=("$D/us_data.csv" $PORT "$D/us_vix" "$D/extra_data/us" "$M1" "$ST/us"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/us-market-data
else PORT=19946; ARGS=("$D/idx_data.csv" $PORT "$D/india_vix" "$D/extra_data/india" "$M1" "$ST/india"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/market-data-repaired-20261015-v2; fi
nice -n 10 "$BIN" "${ARGS[@]}" > "$OUT/server-$MKT.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null; wait $PID 2>/dev/null' EXIT
for i in $(seq 1 120); do curl -sf "http://127.0.0.1:$PORT/api/guru/connors/strategies" -o /dev/null && break; sleep 2; done
if [ "${3:-}" = checks ]; then  # raw-source spot check of entry fills + API status/refusal checks
  timeout 840 nice -n 10 python3 "$HERE/spot_check.py" $PORT "$MKT" "$OUT/spot-$MKT.json" | tee "$OUT/spot-$MKT.txt"
  timeout 300 nice -n 10 python3 "$HERE/api_checks.py" $PORT "$MKT" "$OUT/api-$MKT.json" | tee "$OUT/api-$MKT.txt"
else
  timeout 840 nice -n 10 python3 "$HERE/compare_opt.py" $PORT "$MKT" "$OUT/compare-$MKT.json" ${3:-} | tee "$OUT/compare-$MKT.txt"
fi
