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
| swma | - | swma |
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
| breadth | - | advances, declines, unchanged, adv_volume, dec_volume, unch_volume |
| adl | - | adl |
| ad_ratio | - | ratio |
| cvi | - | cvi |
| trin | - | trin |
| ma_ribbon | type=0, len1=20, len2=50, len3=100, len4=200 | ma1, ma2, ma3, ma4 |
| ma_cross | fast=9, slow=21, fast_type=0, slow_type=0 | fast, slow, cross |
| gmma | - | e3, e5, e8, e10, e12, e15, e30, e35, e40, e45, e50, e60 |
| hamming_ma | length=10 | hamming |
| ama | alpha=0.1 | ama |
| ama2 | alpha=0.1, feedback=0.9 | ama2 |
| iir2 | f0=0.2, f1=1.4, f2=-0.6 | iir2 |
| envelope | length=20, percent=10, exponential=0 | basis, upper, lower |
| ma_channel | upper_length=20, lower_length=20 | upper, lower |
| apz | length=50, mult=2 | basis, upper, lower |
| alligator | jaw_length=13, teeth_length=8, lips_length=5, jaw_offset=8, teeth_offset=5, lips_offset=3 | jaw, teeth, lips |
| median | length=3, atr_length=14, mult=2 | median, upper, lower, ema |
| linreg_slope | length=14 | slope |
| linreg_intercept | length=14 | intercept |
| tsf | length=14 | tsf |
| stderr | length=14 | stderr |
| se_bands | length=21, mult=2, smooth=3 | basis, upper, lower |
| trend_strength | length=14 | tsi |
| rci | length=10 | rci |
| rci_ribbon | short=10, middle=30, long=50 | short, middle, long |
| percentrank | length=20, variant=0 | rank |
| percentile | length=20, pct=50, variant=0 | percentile |
| zscore | length=20 | z |
| skewness | length=20 | skew |
| kurtosis | length=20 | kurt |
| majority_rule | length=14 | pct |
| bop | - | bop |
| bbp | length=13 | bbp, bull, bear |
| accel_osc | fast=5, slow=34, smooth=5 | ac |
| price_osc | fast=10, slow=21, exponential=0 | po |
| woodies_cci | length=14, turbo=6 | cci, turbo |
| rmi | length=20, momentum=5 | rmi |
| cg_osc | length=10 | cg, trigger |
| pmo | length1=35, length2=20, signal=10 | pmo, signal |
| special_k | signal1=100, signal2=100 | sk, signal |
| smi_ergodic | long=20, short=5, signal=5 | smi, signal, osc |
| smi | k=10, d=3, ema=3 | smi, signal |
| rvgi | length=10 | rvi, signal |
| rvi | length=10, ema=14 | rvi |
| chaikin_vol | length=10, roc=10 | cv |
| mass_index | ema=9, length=10 | mi |
| ulcer | length=14 | ulcer |
| adr | length=14 | adr |
| volatility_zt | length=10, days=252 | vol |
| volatility_ohlc | length=10, days=252 | vol |
| bbtrend | short=20, long=50, mult=2 | bbtrend |
| chande_kroll | p=10, x=1, q=9 | stop_long, stop_short |
| chandelier | length=22, mult=3, use_close=1 | long_stop, short_stop, direction |
| vstop | length=20, factor=2 | stop, direction |
| wilder_vi | length=7, mult=3 | sar, direction |
| chop_zone | length=30, ema_length=34 | angle, zone |
| rwi | min_length=9, max_length=40 | hi, lo, rwi |
| asi | limit=10 | asi |
| klinger | fast=34, slow=55, signal=13 | kvo, signal |
| vol | length=20 | volume, ma |
| vwap_period | anchor=1, mult=1 | vwap, upper, lower |
| twap | anchor=1 | twap |
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

## Verification status

Phase 1 ships this harness with fixtures pending; no indicator should be described as TradingView-matched until its fixture passes.

1. Value-verified on TradingView:
   - EMA seeded with the SMA of the first n values (0 differences over 475 bars).
   - SAR's first four NSE:SWIGGY bars (na, 391, 391, 392.968).
2. First-bar-verified only (the first bar with a value matches; later values are not compared):
   EMA5 bar 4, RSI14 14, ATR14 13, DI+ 14, ADX 27, MFI14 13, CMO9 9, OBV 1, HMA9 10, MACD 25 and 33,
   Stoch RSI %K 17, SAR 1, Supertrend(10,3) 9.
3. Everything else: values are NOT yet verified against TradingView. Every indicator in the table above is
   unverified unless it is listed in group 1.

## Export pitfalls

- TradingView's Supertrend shows 0 on bar 0; blank that cell.
- Ichimoku spans and the Pivots built-in are plotted with offsets. Export them from a small Pine script that
  plots the `ta.*` value with no offset.
- Volume must be column 5 when present.
- Delete extra plots.
- The time column is ignored.

## Still needing a fixture

Keltner, TSI, TRIX, DPO, Fisher, KST, Aroon (tie direction), highest/lowest tie rules,
Ichimoku (spans are exported as plotted, calculated displacement-1 bars earlier), Connors RSI, MFI values,
Ultimate Oscillator, ALMA, KAMA, McGinley, Chaikin Oscillator, PVO, EOM, NVI/PVI,
SAR with an outside bar at the start (bar-1 reversal), SWMA.

## Scope notes

Bars since moves to the Phase 2 rule builder (it takes a condition, not a price series).

Breadth fixtures need the market's advance/decline data, not the chart symbol; compare against TradingView `ADVN.NS`/`DECN.NS` style symbols only after confirming they match NSE_ADV_DEC (pending).

## Listed only (no fixture possible)

Entries with `computed:false` in the catalogue (second instrument, intraday/tick, fundamentals, derivatives, on-chain,
drawings, look-ahead functions, unpublished formulas, user coefficient filters) have no fixture. Their catalogue `note`
says why. A rule or chart using one returns `needs_data` or `unsupported` with that note.
