# Indicator parity fixtures (TradingView)

The Indicators lab can be checked against TradingView's own numbers. The fixtures are
CSV files kept outside Git; without them the parity test passes and prints "skipped".

## Making a fixture

1. Open a daily-bar chart of a symbol. Add only one indicator, with the parameters you want to test.
2. Use the chart's "Export chart data" action and save the CSV.
3. Name it `<indicator_id>__<k=v,k=v>.csv`, for example `sma__length=20.csv` or `macd__fast=12,slow=26,signal=9.csv`.
   Parameter names are the lab's own; an indicator with default parameters can omit them.
4. Columns: `time,open,high,low,close[,Volume],<output columns>`. `time` is an ISO date.
   Output columns follow the order of the indicator's outputs in the lab catalogue.
   Empty cells mean "no value" and must match a lab warm-up bar.

## Where to keep them

Anywhere outside the repository (never commit CSVs). Point `LAB_TV_FIXTURES` at the folder.

## Running

```
cd Context/guru/backend
LAB_TV_FIXTURES=/path/to/fixtures CARGO_BUILD_JOBS=2 cargo test lab::parity -- --nocapture
```

Values are compared at 1e-6 relative tolerance because exports are rounded to display precision.

## Already verified on TradingView

EMA uses an SMA seed. First bars with a value: EMA5 bar 4, RSI14 bar 14, ATR14 bar 13,
DI+ bar 14, ADX bar 27, MFI14 bar 13, CMO9 bar 9, OBV bar 1, HMA9 bar 10, MACD bars 25 and 33,
Stoch RSI %K bar 17, SAR bar 1 (SWIGGY start values), Supertrend(10,3) bar 9.

## Still needing a fixture

Keltner, TSI, TRIX, DPO, Fisher, KST, Aroon (tie direction), highest/lowest tie rules,
Ichimoku, Connors RSI, MFI values, Ultimate Oscillator, ALMA, KAMA, McGinley,
Chaikin Oscillator, PVO, EOM, NVI/PVI.
