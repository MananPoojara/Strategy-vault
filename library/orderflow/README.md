# Order-flow footprint data for the Indicators lab

The lab's order-flow indicators (category **Order flow**: Volume Delta, CVD, Up/Down Volume, Delta %, session and composite
Volume Profile, naked POC, POC migration, footprint imbalances, unfinished auctions, delta divergence, max/min row delta,
buy/sell VWAP) compute from one **footprint** file per instrument. Without that file they show **Data needed**.

## File format

`<TICKER with spaces as _>__FOOTPRINT.csv`, for example `NIFTY_50__FOOTPRINT.csv`:

```
date,price,bid_volume,ask_volume
2026-10-23,25810.05,1200,950
```

- One row per session date and price level. `bid_volume` = volume traded at the bid (aggressive sells),
  `ask_volume` = at the ask (aggressive buys). Rows may be in any order; duplicate (date, price) rows are summed.
- Dates `YYYY-MM-DD` (or `DD-MM-YYYY`). Prices on a fixed tick grid.
- A negative or non-numeric volume, or a non-positive price, rejects the whole file (the engine logs why).
- Daily bars only; weekly and monthly charts show Data needed.

## Making it from trades

```
python3 footprint_from_trades.py TRADES.csv "NIFTY 50" OUT_DIR --tick 0.05
```

`TRADES.csv` has `time,price,size` and either `side` (B/S) or `bid,ask`. Trades without a side are classified by the quote
(at/above the ask = buy, at/below the bid = sell, inside the spread by the midpoint) and then by the tick rule
(Lee & Ready 1991). Trades that still cannot be classified are dropped and counted, never guessed.
Self-check: `python3 -I test_footprint_from_trades.py`.

## Pointing the engine at the files

The engine reads `VAULT_ORDERFLOW_DIR` at start and loads each ticker's file once (restart the engine after updating files).
With Docker, mount the folder read-only into `engine-in` / `engine-us` and set the variable, e.g. in `compose.yaml`:

```yaml
    environment:
      VAULT_ORDERFLOW_DIR: /orderflow
    volumes:
      - '/path/to/orderflow:/orderflow:ro'
```
