# Indicators lab — AmiBroker parity gap list

Source: the user's screenshots (`Nandini Patel/Amibroker ScreenShoots.xlsx`, sheets Settings, Analysis, Stats, optimize) and the
AmiBroker guide ([Parameters window](https://www.amibroker.com/guide/w_param.html)). Compared on 2026-10-16 against branch
`Nandini` @ `1b1f5aa` and the working tree (live engine responses from `/api/in/lab/run`, `types.ts`, `api.rs`).

Legend: ✅ have · 🟡 partial · ❌ missing. "Engine" = Rust lab change; "UI" = frontend only. Book code is never touched;
lab-only metrics must not be added to `main.rs` `FULL_COLS` (shared with book optimize; additive-only file).

Status 2026-10-17: Tasks 1–7 done (working tree, not committed). Rows below say what each task built and what is still open.

Status 2026-10-19: stops, money settings, report engine/UI, Scan/Explore, Monte Carlo sizing and custom colour done (working
tree, not committed). Rows updated below; the remaining 🟡 rows say why.

## 1. Chart · indicator Parameters window ("Properties of ASI")

| AmiBroker | Ours | Gap |
|---|---|---|
| Parameters window: property grid, collapsible sections, name \| value rows | Parameters window per chip (`ParamsDialog.tsx`) | ✅ |
| Number params: edit field + slider | edit field + slider | ✅ |
| ParamList / ParamToggle (dropdown, Yes/No) | variant select in the Parameters window | ✅ |
| ParamColor per plot (color picker) | Color per plot: theme palette or "Custom…" `<input type=color>` (`ParamsDialog.tsx`, `Look.color`; a custom hex does not follow light/dark) | ✅ |
| ParamStyle (line, thick, dashed, dotted, hidden …) | Style per plot (subset a line plot can show) | ✅ |
| Reset all · OK; changes apply live | live apply, Reset all, OK | ✅ |
| Preference "Ask for parameters of newly inserted indicators" | a newly added chip opens its Parameters window | ✅ |
| Axes & Grid tab (scaling, log, levels, middle lines) | none | not required (Parameters is the main tab) |
| Title line O/H/L/C + indicator values | TradingView-style legend (2026-10-16) | ✅ |
| Trading arrows | ✅ | ✅ |

## 2. Analysis settings · General

| AmiBroker | Ours | Gap |
|---|---|---|
| Periodicity Y/Q/M/W/D | D/W/M/Q/Y | ✅ |
| Positions Long / Short / Long and short | direction long/short/both | ✅ |
| Initial equity | money mode `execution.capital` | ✅ |
| Min shares, min/max position value, round lot size | `execution.limits` min_shares, min_value, max_value, round_lot (Task 7) | ✅ |
| Tick size | `execution.tick_size` (`ledger::snap_ticks`): rounds only built-in stop / target / take-profit exits, as AmiBroker; futures use the contract tick; single instrument only (RiskPanel General tab) | ✅ |
| Allow position size shrinking | `execution.limits.shrink`: off skips entries a cap or the cash cuts (`insufficient_cash`); without limits the cash shortfall shrinks, as before | ✅ |
| Allow same bar exit / entry signal | `execution.allow_same_bar_exit` (default on); off = no exit on the entry bar and no re-entry on a bar that closed a position (the re-entry half is our reading, disclosed); single instrument only | ✅ |
| Reverse entry signal forces exit | `execution.opposite` reverse/exit/ignore | ✅ |
| Commissions: table / percent / per trade / per share | per_order, per_unit, pct, taxes | ✅ |
| Fixed annual interest on cash | `execution.limits.interest_pct` (idle cash, calendar days) | ✅ |
| Margin rate / account margin | Account margin: `execution.limits.margin_pct` 1–100 (buying power = equity × 100 / margin%, loan earns no idle interest, no margin call). Margin rate (loan interest) not modelled — disclosed in MARGIN_NOTE | 🟡 (margin rate) |
| Pad and align to reference symbol | `execution.pad_align` (`series::pad_align`, reference = market benchmark; padded bars O=H=L=C=prev close, volume 0; disclosed as the first execution event); run, optimize, walk-forward and Explore; not for portfolios or the gl market | ✅ |

## 3. Analysis settings · Trades

| AmiBroker | Ours | Gap |
|---|---|---|
| Buy / Sell / Short / Cover price: Close, Open, High, Low, Average | `execution.prices` per side, all five fields (Task 3) | ✅ |
| Buy / Sell / Short / Cover delay (bars) | `execution.delays` per side, 0..20 (delay 0 needs Close) | ✅ |
| Scenario presets (next bar open / current bar close, stops intraday / next open / close) | Scenario dropdown on the Trades tab; all presets modelled (stop presets set Exit at stop? 2 / 0 for every stop); single instrument only | ✅ |

## 4. Analysis settings · Stops

| AmiBroker | Ours | Gap |
|---|---|---|
| Max loss stop: disabled / percent / point | stop_loss pct / atr / points / ticks | ✅ |
| Profit target: disabled / percent / point | target pct / atr / points / ticks | ✅ |
| Trailing: percent / point / percent of profit | trailing pct / atr / points / `profit_pct` (+activate) | ✅ |
| N-bar stop (+ has priority) | N-bar stop row on the Stops tab (a `bars_held` exit rule) + "Has priority" (`execution.nbar_has_priority`) | ✅ |
| Exit mode per stop (trade price only / exit at stop / next bar) | `execution.exit_at_stop` 0/1/2 per stop (`simulate.rs`): 0 = tested and filled at the trade price, 2 = High/Low trigger, exit next bar at the trade price. Options use AmiBroker's labels (Exit at trade price only (0) / Exit intraday at stop (1) / Exit next bar at trade price (2)); the N-bar row has only Has priority, as in AmiBroker | ✅ |
| Re-entry delay per stop | `execution.reentry` stop_loss / target / trailing / bars (+ `cooldown`) | ✅ |
| Activate stops immediately | `execution.activate_stops_immediately` (Orders & stops tab; lab default on, AmiBroker's default off — disclosed) | ✅ |

## 5. Analysis settings · Report, Portfolio, Walk-Forward, Monte Carlo

| AmiBroker | Ours | Gap |
|---|---|---|
| Risk-free rate for Sharpe, UPI rate | `execution.report.{rf_sharpe, rf_upi}` (defaults 5 / 5.4, echoed as `report.rates`; `ReportSettings.tsx` on the General tab); not for portfolios | ✅ |
| Include buy-and-hold stats | ✅ | ✅ |
| Portfolio: max open positions | portfolio `max_open` | ✅ |
| Limit trade size as % of bar volume | `execution.limits.max_volume_pct` (last complete bar's volume) | ✅ |
| Use previous bar equity for sizing | `execution.limits.prev_bar_equity` | ✅ |
| Walk-forward (easy mode): IS/OOS windows, step, anchored, target metric, IS/OOS table | `/lab/walkforward` + Walk-forward under Optimization (default target CAR/MDD) | ✅ |
| Monte Carlo: N runs from trade list, sizing (don't change / fixed / value / % equity), percentile table, MC equity min/max, drawdown and final-equity CDFs | in-browser Monte Carlo panel: runs, seed, percentile table, equity envelope, CDFs; sizing don't change / % of equity / fixed size / fixed value (`monte-carlo.ts` fixedReturns; money mode only, not compounded, no cash/margin check — disclosed) | ✅ |

## 6. Analysis window (result list, scan, explore, optimize)

| AmiBroker | Ours | Gap |
|---|---|---|
| Trade list: Price, Ex. price, % chg, Profit, % Profit, Shares, Position value, Cum. profit, # bars, Profit/bar, MAE, MFE, Scale in/out | tradesheet with MAE, MFE, shares, position value, cum. profit, profit/bar (Task 2) | ✅ |
| Status line: Profit, CAR, MaxSysDD, CAR/MDD, # winners, # losers | KPI tiles | ✅ |
| Range: all quotes / recent bars / recent days / from–to | from–to, 1Y/3Y/All | ✅ |
| Apply to: current / all symbols / filter | single instrument or portfolio universe | ✅ (portfolio) |
| Scan (signal list), Explore (columns table) | `/api/lab/scan` (`scan.rs`) + `Explore.tsx`: Scan = raw Buy/Sell/Short/Cover list, current or all symbols; Explore = O/H/L/C/V, signals and chart-indicator columns. Raw rule signals, no position state (disclosed). All-symbols scan does not take the gateway optimization slot (engine caps 20k rows, 2M cells, 90 s) | ✅ |
| Optimize: table sortable by any metric | ✅ | ✅ |
| 3D optimization chart | `OptimizeSurface.tsx`: isometric SVG surface or heatmap for two optimized settings; fixed view, not rotatable (disclosed) | ✅ |
| Individual optimize (per symbol) | `/api/lab/optimize?mode=individual` + "Individual optimize" in OptimizePanel (per-symbol best, ≤ 12,000 backtests) | ✅ |
| Info log (timings) | `info {ms, combos, threads, warnings}` in optimize, individual optimize and walk-forward (InfoLog in the UI). A run has none on purpose: elapsed ms would make run output non-deterministic | 🟡 (run) |

## 7. Backtest report (Stats sheet)

| AmiBroker | Ours | Gap |
|---|---|---|
| Columns All trades / Long / Short / Buy&Hold | All / Long / Short / Buy&Hold (`report.buyhold`: one long trade over the run window, same keys as `report.all`) | ✅ |
| Initial / ending capital, net profit, net %, exposure, annual return | ✅ (money mode) | ✅ |
| Net risk-adjusted return, risk-adjusted return (RAR) | `report.system` net_rar, rar | ✅ |
| Winners / losers blocks: total, avg, avg %, avg bars, max consecutive, largest, bars in largest | report winners / losers blocks | ✅ |
| Max trade drawdown ($, %), max system drawdown ($, %) | max_trade_dd, max_sys_dd, max_sys_dd_abs | ✅ |
| Recovery factor, CAR/MDD, RAR/MDD, profit factor, payoff, standard error, risk-reward ratio, Ulcer index, UPI, Sharpe of trades, K-ratio | all in `report` (risk-free rates settable, see §5) | ✅ |
| Charts: portfolio equity, underwater, profit table, profit distribution, MAE/MFE distribution | equity, drawdown, monthly table, profit distribution, MAE and MFE charts | ✅ |
| Report pages: Formula, Settings, Symbols | Statistics / Formula / Symbols pages in `Report.tsx`. Formula is AFL-like text, not real AFL (disclosed). Symbols: per-symbol bar count is a dash for portfolios | ✅ |

## 8. Optimize sheet

AmiBroker's optimization table shows every report metric per combination (No., Net Profit, Net %, Exposure, CAR, RAR, trade and
system drawdowns, recovery, CAR/MDD, RAR/MDD, PF, payoff, std error, RRR, Ulcer, UPI, Sharpe, K-ratio, trade counts and
winner/loser blocks, then the parameter values). Ours shows `FULL_COLS`. Gap: the §7 metrics as optimize columns (lab-only
column list, not `main.rs FULL_COLS`).

## Order of work

1. Parameters window (UI) — the user's first ask.
2. Report metrics + per-direction stats + MAE/MFE (engine) and the report layout (UI).
3. Trades/Stops settings gaps: fill prices High/Low/Average and per-side delays, point stops, trailing % of profit, per-stop
   re-entry delay, scenario presets (engine + UI), arranged as AmiBroker-style settings tabs.
4. Monte Carlo (engine + UI).
5. Walk-forward easy mode (engine + UI).
6. Sizing limits (min shares, round lot, min/max value, % of bar volume, interest on cash).
