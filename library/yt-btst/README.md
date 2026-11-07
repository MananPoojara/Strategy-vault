# yt-btst data generator

`btst_series.py` turns the continuous 1-minute `rolling_futures.parquet` (ts, expiry, open, high, low, close) into the wide CSV `NIFTY_50__BTST_5M.csv`: `Date` plus `O/H/L/C<HHMM>` for 76 AmiBroker start-stamped 5-minute bars (0915..1525 and the lone 1530) = 304 value columns; empty cell = bar missing. One row per futures trading day. Spec: `BTST/BTST_VAULT_SPEC.md` section 2.

Run:

    ~/.venvs/strategy-vault/bin/python btst_series.py --input rolling_futures.parquet \
      --output ~/strategy-vault-data/source/extra_data/india/NIFTY_50__BTST_5M.csv \
      --report ~/strategy-vault-data/btst_series_report.json

Test: `~/.venvs/strategy-vault/bin/python -I -m unittest` from this folder.

Staging: the CSV must live at `~/strategy-vault-data/source/extra_data/india/NIFTY_50__BTST_5M.csv` (git-ignored), then run `python3 Context/guru/webapp/scripts/setup.py` and restart engine-in.

Caveats:
- Prices are Nifty front-month futures, not spot. The engine ticker `NIFTY 50` only supplies the calendar.
- Contracts are not stitched or adjusted; the roll dates (expiry changes) show as jumps. They are listed in the report.
- A bar stamped t covers minutes t..t+4; close = close of minute t+4 (AmiBroker "14:50" = close of the 14:54 minute). Partial bars use the minutes that exist (close = last existing minute) and are counted in the report.
- Muhurat/evening-session timestamps outside 09:15-15:30 are dropped and listed in the report.
