# Indicator parity fixtures (TradingView)

The Indicators lab can be checked against TradingView's own numbers. The fixtures are
CSV files kept outside Git; without them the parity test passes and prints "skipped".

## Making a fixture

1. Open a daily-bar chart of a symbol. Add only one indicator, with the parameters you want to test.
2. Use the chart's "Export chart data" action and save the CSV.
3. Name it `<indicator_id>__<k=v,k=v>.csv`, for example `sma__length=20.csv` or `macd__fast=12,slow=26,signal=9.csv`.
   Parameter names are the lab's own; an indicator with default parameters can omit them.
4. Columns: `time,open,high,low,close[,Volume],<output columns>`. The first column (time) is ignored:
   unix seconds or ISO dates both work. Include `Volume` only for indicators that need it.
5. Output columns are matched by position, not by header name. After the time/OHLC[/Volume] columns, keep
   exactly the indicator's outputs, in the order of the table below. Delete any extra TradingView plots
   (band fills, signal lines the lab does not output, etc.) and reorder the rest.
   Empty cells mean "no value" and must match a lab warm-up bar.
   `pivot_high` and `pivot_low` outputs are sparse: NaN between pivots is expected.

## Indicator ids, parameters and output order

| id | parameters (default) | outputs in order |
|---|---|---|
| sma | length=9 | sma |
| ema | length=9 | ema |
| wma | length=9 | wma |
| smma | length=7 | smma |
| vwma | length=20 | vwma |
| hma | length=9 | hma |
| alma | length=9, offset=0.85, sigma=6 | alma |
| lsma | length=25 | lsma |
| dema | length=9 | dema |
| tema | length=9 | tema |
| kama | length=10, fast=2, slow=30 | kama |
| mcginley | length=14 | mg |
| rsi | length=14 | rsi |
| macd | fast=12, slow=26, signal=9 | macd, signal, hist |
| stoch | k=14, smooth_k=1, d=3 | k, d |
| stochrsi | k=3, d=3, rsi_length=14, stoch_length=14 | k, d |
| cci | length=20 | cci |
| mom | length=10 | mom |
| roc | length=9 | roc |
| willr | length=14 | willr |
| mfi | length=14 | mfi |
| tsi | long=25, short=13, signal=13 | tsi, signal |
| cmo | length=9 | cmo |
| uo | fast=7, mid=14, slow=28 | uo |
| ao | fast=5, slow=34 | ao |
| ppo | fast=12, slow=26, signal=9 | ppo, signal, hist |
| trix | length=18 | trix |
| kst | roc1=10, roc2=15, roc3=20, roc4=30, sma1=10, sma2=10, sma3=10, sma4=15, signal=9 | kst, signal |
| dpo | length=21 | dpo |
| coppock | wma=10, long=14, short=11 | coppock |
| fisher | length=9 | fisher, trigger |
| crsi | rsi=3, streak=2, rank=100 | crsi |
| dmi | di_length=14, adx_smoothing=14 | plus_di, minus_di, adx |
| supertrend | atr_length=10, factor=3 | supertrend, direction |
| sar | start=0.02, increment=0.02, max=0.2 | sar |
| aroon | length=14 | up, down |
| aroon_osc | length=14 | osc |
| ichimoku | conversion=9, base=26, span_b=52, displacement=26 | tenkan, kijun, span_a, span_b |
| vortex | length=14 | vi_plus, vi_minus |
| chop | length=14 | chop |
| atr | length=14 | atr |
| bb | length=20, mult=2 | basis, upper, lower |
| bbpct | length=20, mult=2 | pct_b |
| bbw | length=20, mult=2 | width |
| kc | length=20, mult=2, atr_length=10 | basis, upper, lower |
| donchian | length=20 | upper, lower, basis |
| stdev | length=20 | stdev |
| hv | length=10 | hv |
| obv | - | obv |
| accdist | - | ad |
| cmf | length=20 | cmf |
| chaikin_osc | fast=3, slow=10 | osc |
| pvt | - | pvt |
| pvo | fast=12, slow=26, signal=9 | pvo, signal, hist |
| efi | length=13 | efi |
| eom | length=14, divisor=10000 | eom |
| nvi | ema=255 | nvi, signal |
| pvi | ema=255 | pvi, signal |
| highest | length=20 | highest |
| lowest | length=20 | lowest |
| change | length=1 | change |
| pivot_high | left=5, right=5 | pivot |
| pivot_low | left=5, right=5 | pivot |

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
