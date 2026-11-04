#!/bin/bash
# K39: start one private engine (us|in) with the 1-minute dir on its own port, run a python script against it, stop it by PID.
# usage: authors/heavy K39w/run_engine.sh BIN us|in script.py [args...]   (the script gets PORT as its last argument)
BIN=$1; MKT=$2; shift 2; D=/home/server/.cache/strategy-vault-webapp/data; W=$(dirname "$(readlink -f "$0")")
if [ "$MKT" = us ]; then P=19961; A=("$D/us_data.csv" $P "$D/us_vix" "$D/extra_data/us" "$D/intraday"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/us-market-data
else P=19962; A=("$D/idx_data.csv" $P "$D/india_vix" "$D/extra_data/india" "$D/intraday"); export VAULT_BASKET_DATA_DIR=/home/server/.cache/strategy-vault/market-data-repaired-20261029-v3; fi
cd $D || exit 1
nice -n 10 "$BIN" "${A[@]}" > "$W/server-$MKT.log" 2>&1 & PID=$!
trap 'kill $PID 2>/dev/null; wait $PID 2>/dev/null' EXIT
for k in $(seq 1 300); do curl -sf -o /dev/null "http://127.0.0.1:$P/api/guru/connors/strategies" && break; sleep 2; done
nice -n 10 ionice -c3 python3 "$@" $P
