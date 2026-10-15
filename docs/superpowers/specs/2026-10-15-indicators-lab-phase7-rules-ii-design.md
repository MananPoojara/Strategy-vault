# Indicators lab Phase 7 "Rules II": entry and exit conditions, orders and instruments (design)

Status: draft for the user's review · Branch: `Nandini` (from `Manan`) · Date: 2026-10-15
Parent specs: `2026-10-14-indicators-lab-design.md` (§4 rule spec, §7 errors, §9 delivery) and
`2026-10-16-customize-book-strategies-design.md` (§5 engine additions, §5.1 bar event order, §5.2 no lookahead).
Runs after Phase 6 (HEAD `53d7308` at writing). Plan: `docs/superpowers/plans/2026-10-15-indicators-lab-phase7-rules-ii.md`.

## 1. Goal

The user asked tonight: *"implement all the possible entry exit conditions available in rules … add all the
possible ways of entry exit possible; we will be running on spot, futures and equity so properly"*. The user
could not be asked questions, so every open point was decided here and is listed in §13.

Phase 7 makes the lab rule spec express every entry and exit condition that AmiBroker AFL or a TradingView Pine
strategy can express **on daily bars of one instrument**. It also adds the order types, exit types, re-entry
rules and risk rules of both platforms, plus an explicit execution model for each instrument kind:

- cash equity (India and US stocks and ETFs);
- index and spot (signal studies only; never traded as a substitute for something else);
- futures (a full contract model, tested on fixtures; real requests answer `needs_data` until contract data
  is admitted).

Also in scope (added by the user on 2026-10-15): AmiBroker's higher-timeframe semantics for rules
(TimeFrameSet / Compress / Expand, §5.1).

Out of scope: intraday bars, options, user code and new book templates (§12 lists the follow-ups). Multi-symbol
features (Foreign operands, foreign entry and exit, ranking, rotation and portfolio) are Phase 8, planned
separately. Phase 7 stays single-instrument and leaves the `symbol` seam (§5.1).

## 2. Constraints carried from AGENTS.md and the earlier specs

- **Book results are never touched.** Protected files get no edits: `connors.rs`, `books/**`, `readiness.rs`,
  `gate.rs`, `instrument_policy.rs`, `baskets.rs`, `accounting.rs`, `trade_validation.rs`, frontend `compat/*`
  and `ResearchPages.tsx`. `main.rs`, gateway `policy.rs` and `custom-pages.ts` are additive-only, and **this
  phase changes none of them**. `scripts/check_book_untouched.py --engine-compare` must pass at the gate.
- **Every existing spec keeps its exact meaning.** This covers v1 specs, the 43 Customize templates and saved
  strategies (§8).
- **Keep four things separate:** verified source rules, declared adaptations, simulated results and
  book-reported figures. A documented-example fixture (§10.3) reproduces *semantics*; it verifies nothing
  about a book.
- **No result where the rules or data are unresolved.** Futures without admitted contract data, money sizing
  on an index, and an India cash short in money mode all answer `needs_data`, never a number (§7).
- **Stock methods need actual stocks.** An index never stands in for a stock, an ETF or a futures contract,
  and a cash index never stands in for futures prices.
- **Working rules:** branch `Nandini`; never `main`; never push (the user pushes); no worktrees; no
  `Co-Authored-By` trailer; `CARGO_BUILD_JOBS=2`; std only (no new crates); the NAS is read-only; no
  whole-corpus backtest; no data, `.env` or results in Git.

## 3. What exists today (HEAD after Phase 6)

- **Rule spec v1** (`lab/rules.rs`):
  - Conditions `{lhs, op, rhs}` with the operators `crosses_above`, `crosses_below`, `>`, `<`, `>=` and `<=`.
  - Operands: a price source, a catalogue indicator output (`s`, `p`, `out`, `tf` D|W|M, `offset` ≥ 0), a
    number, or `bars_held` (exit only).
  - `entry.all` (all of), `exit.any` (any of), at most 12 conditions in total, and specs of at most 8,192 bytes.
  - Customize fields: `direction` long|short, `origin`, `execution.fill` / `exit_fill` (next_open | close),
    `entry.limit`, and `exit.stop_loss` / `target` / `trailing` (pct or ATR). Optimize paths reach these.
  - Reserved fields that answer `unsupported`: `spec.portfolio`, `spec.scale_in`, `spec.short`,
    `entry.stop`, `entry.scale_in`, and the fill values `limit` and `stop`.
- **Simulator** (`lab/simulate.rs`): one position at a time. Every trade is a book-format `connors::Trade`
  (legs with weights, a scale-out ledger `exits`), so book `trade_validation`, `equity_curve` and `stats` are
  reused read-only. Bar event order is as in the Customize spec §5.1.
- **Accounting** (book, read-only): a normalized **unit allocation**. Returns are a percentage of one unit;
  costs are bps per side; legs must sum to at most 1; `capital` is refused ("percentage-based report").
- **Catalogue:** 299 indicators, 183 of them computed, including `calendar` (dow, day, month, year, doy,
  bar_index), `mtp`, `performance` and `bars_since_extreme`. Phase 6 marked the AFL utilities `BarsSince`,
  `ValueWhen`, `HighestSince`, `Sum`, `IIf`, `ExRem` and `Flip` as "rule-builder follow-up". This phase is
  that follow-up.
- **Gateway:** engine routes are GET-only. Queries are capped at 16,384 bytes, the lab spec at 8,192 bytes,
  and top-level `entry.all` + `exit.any` at 12. Lab optimize is single-slot and restricted to research roles.
- **Engine tests at HEAD:** 848 in total, 276 of them lab tests (measured on a scratch copy of `53d7308`).

### 3.1 Data reality (read-only look, `ls`/`head` only, 2026-10-15)

| Need | What exists | Admitted? | Consequence |
|---|---|---|---|
| Index daily bars (India) | `idx_data.csv`: NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT | yes | signal studies (§7.2) |
| Global/crypto | `Global Bhavcopy/*.csv` (TradingView exports, Phase 5) | provisional | signal studies, never publishable |
| Equity (India) | Live-server basket cache (`VAULT_BASKET_DATA_ROOT`, `/home/server/.cache/strategy-vault/market-data`): NSE 500 (499), Nifty 50 (50), Midcap 100 (100), F&O cash stocks (224); per `library/market-data/README.md` | yes (live server only; absent on this workstation) | money mode on actual stocks (§7.1) |
| Equity (US) | `VAULT_US_BASKET_DATA_ROOT` basket cache, plus SPY/QQQ/IWM/MDY/SMH/DIA ETFs | yes (live server) | money mode, with shorts allowed if borrow cost is declared |
| Corporate actions | Native AmiBroker series; the README says the import "does not … prove corporate-action treatment" | — | quantities are labelled "on the admitted series; adjustment status not established" (§7.1) |
| Futures contracts | Share-level `DATABASE/BHAVCOPY/FO BHAVCOPY` (36 daily files, April 2022 only: SYMBOL, EXPIRY_DT, OHLC, VAL_INLAKH, OPEN_INT; no lot size, no instrument type). `DATABASE/Cuurent and Next Month Continuous Futures` (Nifty/BankNifty stitched current and next month from 2011, with no contract identity per row). `Future Data of Diff Expiry for NF & BNF` (xlsx). `#Future` workbooks | **no** (no lot table, no tick size, no expiry calendar, no admission pipeline) | futures model implemented and tested on fixtures; real requests answer `needs_data` (§7.3) |
| Perpetual/funding (crypto) | none | no | crypto stays long-only spot study; shorts labelled hypothetical (§7.2) |
| Holiday calendar | none (sessions are known only from bar dates) | — | the `sessions_left` indicator (outputs `m`, `q`, `y`) is a declared calendar adaptation (§5.2) |

## 4. Condition vocabulary (entry and exit)

### 4.1 Grammar (additive to v1)

A rule list (`entry.all`, `exit.any`, and the same lists in `short_side`) holds **nodes**. Each node is
exactly one of the following.

| Node | JSON | True on bar *i* when | AFL / Pine equivalent |
|---|---|---|---|
| comparison (v1) | `{"lhs":A,"op":OP,"rhs":B}` | `OP` holds at *i* | `>`, `Cross`, `ta.crossover` … |
| between | `{"lhs":A,"op":"between","lo":L,"hi":H}` | L ≤ A ≤ H | `A >= L AND A <= H` |
| outside | `{"lhs":A,"op":"outside","lo":L,"hi":H}` | A < L or A > H | |
| rising / falling | `{"lhs":A,"op":"rising","bars":n}` | A[i] > A[i−1] > … > A[i−n] (strict) | `ta.rising(A,n)` (Pine compares with the max) see §4.4 |
| all / any | `{"all":[nodes]}`, `{"any":[nodes]}` | every / some child true | `AND` / `OR` |
| not | `{"not":node}` | child false | `NOT` |
| at least k of m | `{"at_least":k,"of":m,"when":node}` | node true on ≥ k of bars i−m+1..i | `Sum(cond, m) >= k` |
| sequence | `{"then":[n1,n2(,n3,n4)],"within":N}` | the last step is true at *i*, and earlier steps were true on strictly earlier bars in order, the first no more than N bars before *i* | `BarsSince` chains |

`OP` gains `=`, `!=` and `crosses` (either direction) next to the six v1 operators. A comparison accepts
`"rhs"` as before.

**Operands** (each `A`, `B`, `L`, `H` above) are v1 operands (series, number, `bars_held`) or one of the
expression objects below. Every expression is evaluated on the daily bars. Higher-timeframe inputs arrive
already expanded after their period closes (§5.1), so arithmetic on mixed timeframes is causal.

| Expression | JSON | Value at *i* | AFL / Pine |
|---|---|---|---|
| arithmetic | `{"add":[A,B]}`, `sub`, `mul`, `div`, `min`, `max` | A op B (div by 0 → no value) | `+ - * /`, `Min`, `Max` |
| absolute | `{"abs":A}` | \|A\| | `abs` |
| percent difference | `{"pct":[A,B]}` | 100·(A − B)/B | `(A/B − 1)*100` |
| shift | `{"ref":A,"bars":n}` | A[i−n] (n ≥ 0) | `Ref(A,−n)`, `A[n]` |
| window | `{"sum":A,"bars":n}`, `avg`, `highest`, `lowest` | over bars i−n+1..i | `Sum`, `MA`, `HHV`, `LLV`, `math.sum`, `ta.highest(src)` |
| change | `{"change":A,"bars":n}` | A[i] − A[i−n] | `A − Ref(A,−n)`, `ta.change` |
| bars since | `{"bars_since":node}` | bars since the node was last true (0 if true now) | `BarsSince`, `ta.barssince` |
| value when | `{"value_when":node,"of":A,"nth":k}` | A on the k-th most recent bar where node was true (k ≥ 1) | `ValueWhen`, `ta.valuewhen` |
| highest / lowest since | `{"highest_since":node,"of":A}` | max (min) of A from the latest bar where node was true through *i* | `HighestSince`, `LowestSince` |
| count | `{"count":node,"bars":n}` | number of bars in i−n+1..i on which node was true | `Sum(cond,n)` |
| compress | `{"compress":A,"tf":T,"mode":M,"expand":E}` | A compressed to timeframe T, then expanded (§5.1) | `TimeFrameCompress` + `TimeFrameExpand` |

Notes:

- `{"ref":…}` on an expression is the general form of v1's operand `offset`. Offsets stay non-negative; a
  negative or future shift is refused (AFL `Ref(x, +1)` looks ahead).
- `bars_held` stays a **top-level exit operand only**, as in v1. It is refused inside groups, sequences and
  expressions, because its value depends on the open position and those nodes are precomputed.
- Constants are `{"value":x}`, as in v1. A comparison of two numbers is refused, as in v1.

### 4.2 Truth values and missing data (three-valued)

Inside nodes, a leaf is **unknown** when any input it reads is missing (NaN), including the previous bar for
crosses, the whole window for `rising`, and the `lo`/`hi` bounds. The rules are:

- `all` is false if any child is false, otherwise unknown if any child is unknown, otherwise true;
- `any` is true if any child is true, otherwise unknown if any child is unknown, otherwise false;
- `not` of unknown is unknown;
- the counting nodes follow Kleene three-valued logic, so a warm-up never turns into a false that a `not` could
  flip: `at_least k of m` is true if the true bars in the last m reach k, false if true plus unknown bars cannot
  reach k, otherwise unknown, and unknown until m bars exist; `then` is true when a chain is found, otherwise
  unknown if any step was unknown on a bar of the last `within`+1 bars (and its last step is true), otherwise false,
  and unknown while its last step is unknown; `count`, `bars_since`, `value_when` and the `*_since` operands follow
  the same rule (a missing answer that an unknown bar could have changed is unknown, not 0 or "never").

At the top of a rule list, unknown counts as **false**, so a node never fires during warm-up. A top-level v1
comparison keeps its exact v1 code path, where NaN gives false. Example: `not(close > sma(200))` does not fire
on the first 199 bars. AmiBroker would treat Null there in an undocumented way; TradingView's `not na` would
fire.

Expressions return NaN when any input is NaN, and division by zero gives NaN. Window expressions are NaN while
the window is incomplete or holds a NaN. `highest_since` and `lowest_since` are NaN until the node has been
true once, and unknown (NaN) when A is missing on any bar from the start bar to the current bar (Kleene, as
above). `value_when` is NaN until the k-th occurrence.

### 4.3 Limits (decided; §13 items 13–15)

| Bound | v1 | Phase 7 |
|---|---|---|
| Spec bytes | 8,192 | 8,192 (unchanged; the gateway enforces the same cap) |
| Top-level items, `entry.all` + `exit.any` | 12 | 12 (unchanged; the gateway counts exactly this) |
| Top-level items, `short_side.entry.all` + `short_side.exit.any` | — | 12 |
| Leaf comparisons in the whole spec (nested, in state operands, both sides) | 12 | **40** |
| Nodes + expressions in the whole spec | — | **120** |
| JSON nesting | 16 (`json.rs`) | 16 (unchanged; deep specs fail as "nesting too deep") |
| Window, shift, `bars`, `within`, `of` | offset ≤ 1000 | 1..1000 (shift 0..1000) |
| `nth` | — | 1..20 |
| Sequence steps | — | 2..4 |

GET stays the only method for engine routes; the gateway is unchanged. A spec that URL-encodes past 16,384
bytes is refused by the gateway's existing query check. The catalogue response publishes `max_spec_bytes`,
`max_query_bytes` (16384), `max_leaves` and `max_nodes`, so the later frontend can warn before sending.

### 4.4 Platform conventions this vocabulary follows

- `crosses_above` stays a > b with a[1] ≤ b[1], as in v1, AFL `Cross` and Pine `ta.crossover`.
- `rising` / `falling` are a strictly monotone run of n bars. Pine `ta.rising(src, n)` instead means
  "src > the highest of the previous n values". The two definitions differ. The Pine meaning can be written
  with expressions: `{"lhs":A,"op":">","rhs":{"highest":{"ref":A,"bars":1},"bars":n}}`. The catalogue note
  states both.
- `bars_since` counts 0 on the true bar, as AFL `BarsSince` and Pine `ta.barssince` do.
- `value_when` with `nth: 1` is the latest occurrence, as AFL `ValueWhen(…, 1)` and Pine
  `ta.valuewhen(…, 0)` are.
- `IIf`, `Flip` and `ExRem` are not operands. ExRem-style cleaning is the simulator's default (§6.4), and
  `entry_on: "edge"` gives the cross-only form. `Flip` is a position state, which the simulator already holds.

## 5. Timeframes and calendar

### 5.1 Higher timeframes with AmiBroker TimeFrame semantics (user request, added 2026-10-15)

The user asked to "replicate AmiBroker [behaviour] with rules — higher timeframe". This section maps
AmiBroker's documented functions onto operand fields. The sources are the "Multiple time frame support" page
(https://www.amibroker.com/guide/h_timeframe.html) and the reference pages for
`TimeFrameSet` (https://www.amibroker.com/guide/afl/timeframeset.html),
`TimeFrameCompress` (https://www.amibroker.com/guide/afl/timeframecompress.html),
`TimeFrameExpand` (https://www.amibroker.com/guide/afl/timeframeexpand.html) and
`TimeFrameGetPrice` (https://www.amibroker.com/guide/afl/timeframegetprice.html).

| AmiBroker | Lab rule spec | Notes |
|---|---|---|
| `TimeFrameSet(inWeekly)` … indicator … `TimeFrameRestore()` | operand `"tf":"W"` (v1) | The indicator runs on the compressed bars: O = first, H = max, L = min, C = last, V = sum (`resample.rs`). |
| `inMonthly`, quarterly, yearly | `"tf":"M"`, **`"Q"`, `"Y"`** | Q and Y are new: calendar quarters and years, from `resample::period_key` anchors 3 and 4. |
| N-day custom interval (`TimeFrameSet(n*inDaily)`) | **`"tf":"2D"` … `"63D"`** | Consecutive blocks of n sessions counted from the instrument's **first admitted bar**, not from the run window, so a block never depends on `from`. The guide does not document AmiBroker's own anchoring, so this is a declared adaptation. |
| `TimeFrameCompress(array, interval, mode)` | **expression `{"compress":A,"tf":"W","mode":"last|first|high|low|sum"}`** | Compresses any daily operand or expression (`compressLast`, `compressOpen`, `compressHigh`, `compressLow`, `compressVolume`). The result is a higher-timeframe array that is then expanded. |
| `TimeFrameExpand(x, tf, expandLast)` (the default) | **`"expand":"last"`** (the default, = v1 behaviour) | The value of period p is used from the last daily bar of p, after its close, until the next period's last bar. |
| `TimeFrameExpand(x, tf, expandPoint)` | **`"expand":"point"`** | Only on the period's last daily bar; NaN elsewhere. |
| `TimeFrameExpand(x, tf, expandFirst)` | **`"expand":"first"`, only where it is lookahead-free** | The value of period p is shown from p's first daily bar. AmiBroker warns that this "looks into the future" except for the open. The lab accepts it only where the value is known by the close of p's first bar: (a) `offset` ≥ 1 (a completed earlier period; the same as TradingView's non-repainting `request.security(…, expr[1], lookahead_on)` idiom); (b) the price source `open` with `offset` 0; (c) `compress` with `mode:"first"`. Any other `expand:"first"` answers `unsupported`: "expandFirst of <operand> would show the period's final value from its first day (lookahead); use offset ≥ 1 or expand last". |
| `TimeFrameGetPrice("C", inWeekly, -1)` | `{"s":"close","tf":"W","offset":1,"expand":"first"}` | AmiBroker's default mode for `TimeFrameGetPrice` is expandFirst, so a shift of 0 looks ahead. The lab refuses that case by rule (c) above. |

Unchanged rules: `offset` counts bars of the operand's own timeframe (v1); the last, unfinished period shows
its developing value under `expand:"last"`, as W and M do today; `"tf":"D"` takes no `expand`.

Declared: `expand:"point"` and `compress` (like `last`) also show the unfinished last period's developing value
on the final data bar, because that bar is the period's latest known session; the value can change when the
period completes. `nD` blocks count bars, so on IG CFDs (which carry Sunday bars) a Sunday bar counts as a
session.

Each mode has a dedicated no-lookahead test (§10.1), with these expected values:

- `last`: the value of period p is invisible before p's last bar;
- `point`: NaN on every non-final bar;
- `first` with offset 1: equals the previous period's value from the first bar of p;
- `first` on `open`: equals p's first daily open from p's first bar;
- every refused `first` combination answers `unsupported`.

The existing generic tests (`signals_never_use_later_bars`, `every_indicator_at_parameter_bounds_never_panics`)
are extended to Q, Y and `5D` for every computed indicator, and are run once per expand mode on a price
source.

**Phase 8 seam (multi-symbol).** A series operand reserves the field `"symbol"`. In Phase 7 it answers
`unsupported`: "operands from another symbol (AmiBroker Foreign / TradingView request.security on another
ticker) arrive in Phase 8". Every operand is resolved through one function, `series::series(fr, cache, s)`,
which reads only the `Frames` of the run's instrument. Phase 8 can add a per-symbol `Frames` lookup there
without touching the rule grammar.

### 5.2 Calendar operands (new catalogue indicators `session` and `sessions_left`)

`calendar` (Phase 6) already gives day of week, day of month, month, year and day of year. Phase 7 adds two
indicators. `session` has these outputs (all causal; each counter is NaN until the data's first boundary of
its period, because the data's first, partial period does not show where that period began; `week_of_month`
waits for a month boundary):

| Output | Meaning | Causal? |
|---|---|---|
| `tdom`, `tdoq`, `tdoy` | trading-day number in the month / quarter / year (1 = first session) | yes |
| `tdow` | trading-day number in the week | yes |
| `week_of_month` | 1 + number of earlier week starts in the month | yes |
| `quarter` | 1–4 | yes |
| `weekdays_left_m`, `weekdays_left_q`, `weekdays_left_y` | Monday–Friday dates after today up to the period end (holidays unknown) | yes (pure date arithmetic) |

`sessions_left` is a separate indicator with outputs `m`, `q` and `y`: sessions after today in the month /
quarter / year, from the instrument's own **session dates**, NaN in the final, unfinished period of the data.
It uses future session *dates*, never future prices: a declared adaptation. Every run, optimize or series
response that reads it carries a top-level `"adaptations"` note saying so (the key is absent otherwise).

"First N trading days of the month" is `tdom <= N`. "Last N trading days" is `sessions_left.m < N`, which
matches what a trader with the published exchange holiday list would know. The generic truncation tests exempt
only `sessions_left`; its own property test shows that, under every cut, each finite value equals the
full-data value, and a dedicated test shows it depends on dates alone (§10.1). `weekdays_left_*` is the strictly causal
alternative: it misses the period's last session when that session's weekday is a holiday.

Futures expiry operands come from a third new indicator, `expiry`, with these outputs:

- `days_to_expiry`: Monday–Friday dates until the held contract's expiry date;
- `expiry_week`: 1 in the expiry's calendar week;
- `expiry_day`: 1 on the expiry date.

All three are computed from the expiry date, which is published contract metadata, so they are causal. Only a
futures run carries expiry dates (§7.3). Elsewhere `expiry` answers `needs_data` ("needs futures contract data"). Time exits at a calendar boundary are plain exit
conditions with these operands, for example `sessions_left.m = 0` with `exit_fill: close`. No separate
simulator field is needed.

## 6. Execution and orders

### 6.1 Spec fields (all optional; the defaults reproduce v1 and Customize exactly)

```json
{ "v": 1, "direction": "long | short | both",
  "entry": { "all": [nodes], "limit": D, "stop": D, "valid_bars": 1,
             "scale_in": { "legs": [0.5, 0.25, 0.25], "min_bars": 1, "price": "any | better | worse" } },
  "exit":  { "any": [nodes], "stop_loss": D, "target": D,
             "trailing": { "pct": 8, "activate": D }, "breakeven": { "after": D, "plus": D },
             "take_profits": [ { "pct": 5, "size": 0.5 } ], "min_bars": 0 },
  "short_side": { "entry": {…same shape…}, "exit": {…same shape…} },
  "execution": { "fill": "next_open | close", "exit_fill": "…", "delay_bars": 1,
                 "entry_on": "level | edge", "opposite": "reverse | exit | ignore",
                 "intrabar": "stop_first | ohlc_path",
                 "cooldown": { "bars": 3, "after": "any | loss | win" },
                 "max_trades": { "count": 2, "per": "W | M | Q | Y" },
                 "risk": { "max_consecutive_losses": 4, "pause_bars": 20, "max_drawdown_pct": 25 },
                 "sizing": { "pct_equity": 100 } | { "risk_pct": 1 } | { "units": 10 } | { "value": 100000 },
                 "capital": 1000000, "costs": {…§7.4…}, "cost_bps": 12,
                 "instrument": { "kind": "cash | futures", "roll": {…}, "adjust": "forward | ratio | none" } },
  "optimize": [ … ] }
```

**Distance `D`** is `{"pct":p}`, `{"atr":m,"atr_length":n}` (v1), or one of the new forms: `{"points":x}`
(price units), `{"ticks":n}` (futures only: n × the contract tick) and `{"at":operand}`. The `at` form is for
entry orders (`entry.limit`, `entry.stop`) only, and gives an absolute level taken from an operand's value at the
signal bar. An example is a channel-breakout stop entry at `highest(high,5)`. `breakeven.plus` takes
pct | atr | points; an `at` level there is refused (ruling 2026-10-15, batch 3 review), and `plus` must be below
`after` when both use the same units.

| Field | Meaning | Platform reference |
|---|---|---|
| `direction: both` | Long rules in `entry`/`exit`, short rules in `short_side`; one position at a time | AFL Buy/Sell/Short/Cover; Pine long and short entries |
| `execution.opposite` | when the other side's entry fires while in a position: `reverse` (default) closes and opens in one fill; `exit` only closes; `ignore` does nothing | AB `ReverseSignalForcesExit` (default on); Pine `strategy.entry` reverses, `strategy.risk.allow_entry_in` only closes |
| both entries true while flat | **no entry** (ambiguous) | AB "Buy beats Short"; Pine depends on code order (§13 item 20) |
| always in | `direction: both`, `opposite: reverse`, empty exits | AFL `Short = Sell; Cover = Buy;` |
| `entry.stop` | stop (breakout) entry above the signal close for a long (below for a short), or at an `at` level; fills when the high reaches it, at the worse of open and level | Pine `strategy.entry(stop=)`; AFL `BuyPrice = Max(O, level)` |
| `entry.stop` + `entry.limit` | stop-limit: the stop arms a limit (§6.2). A limit given as a distance (pct / points / atr) always sits below the signal close for a long (above for a short), so such a stop-limit fills only after a gap through the stop; use an `at` limit for the usual buy stop-limit (limit ≥ stop) | Pine `strategy.entry(stop=, limit=)` |
| `entry.valid_bars` | how many bars a stop / limit / stop-limit order rests (1..20; default 1 = v1) | Pine orders persist until cancelled; GTC-N |
| `execution.delay_bars` | next-open market orders fill at the open of bar i+k (1..10; default 1) | AB `SetTradeDelays(k,…)` |
| `execution.entry_on: edge` | enter only on a bar where the entry rule set turns true (it was false or unknown on the previous bar) | Pine `crossover`-style triggers |
| `exit.trailing.activate` | trailing arms only after the best price (through the previous bar) has moved `activate` in favour | AB `ActivationFloor`; Pine `trail_points` / `trail_price` |
| `exit.breakeven` | once the best price is `after` in favour, the stop becomes at least entry ± `plus` | common EA rule; AB via a volatile stop |
| `exit.take_profits` | up to 3 partial exits, each a fraction of the **initial** position at its level | Pine `qty_percent` (of the initial size); AB `sigScaleOut` |
| `exit.min_bars` | rule exits are ignored until bars_held ≥ n; **price exits stay active** | AB `HoldMinBars` blocks every exit (§13 item 22) |
| `execution.cooldown` | no new entry within n bars after an exit (of any trade, or a losing / winning one) | AB `ApplyStop` ReEntryDelay |
| `execution.max_trades` | at most `count` entries per calendar W/M/Q/Y period | |
| `execution.risk` | after n consecutive losing trades, no entries for `pause_bars`; closed-trade equity drawdown ≥ x% stops new entries for good | Pine `strategy.risk.max_cons_loss_days`, `max_drawdown` |
| `execution.intrabar` | which in-bar event comes first (§6.3) | AB stop priority vs Pine O-H-L-C path |
| `entry.scale_in` | adds legs while in a trade when the entry rule set is true again, at least `min_bars` after the previous leg, and optionally only at a better or worse price than the last leg. Leg weights sum to ≤ 1 | AB `sigScaleIn`; Pine `pyramiding` |
| `execution.sizing` | position size (§7.4); unit mode accepts `pct_equity` and `risk_pct` only | AB `SetPositionSize`; Pine `default_qty_type` |

Validation, each failure naming the field: `short_side` only with `both`, and required then; `take_profits`
sizes in (0,1) with a sum below 1; `scale_in` legs sum ≤ 1, and it cannot be combined with `take_profits`
(the book trade ledger defines scale-out fractions of a single position); `risk_pct` needs a `stop_loss`;
`ticks` only for futures; `at` only where allowed; `execution.fill` cannot be combined with an `entry.limit`
or `entry.stop` order (v1 rule, extended).

### 6.2 Bar event order (extends Customize §5.1)

On bar *i*:

1. **Open.**
   1. A pending next-open exit fills, including the closing half of a reversal.
   2. Pending next-open market entries fill: a new position, the opening half of a reversal, or a scale-in leg.
   3. Resting entry orders are tested in this order: limit (low ≤ L, fill at min(open, L)), stop (high ≥ S,
      fill at max(open, S)), stop-limit. Short orders mirror these. While a position is open, a resting order
      for the opposite side is a **reversal order**: it is tested in step 2 together with the position's
      price exits.
2. **Inside the bar**, in the order given by `execution.intrabar` (§6.3): the protective level, the target,
   take-profits and reversal orders.
   - The protective level is the tighter of stop-loss, breakeven and an armed trailing level.
   - A gap through a level fills at the open.
   - At most one position change happens inside a bar. The exception is a reversal order, which closes and
     opens in one fill.
3. **Close.**
   1. Rule exits (`any`, after `min_bars`) fill at this close or set a next-open exit.
   2. The opposite side's entry is applied (`reverse` / `exit` / `ignore`).
   3. Scale-in signals are checked.
   4. New entries are checked (all of; gated by `entry_on`, the conflict rule, cooldown, `max_trades` and risk
      pauses). Each fills at this close, sets a next-open order, or places a resting order for bars
      i+1..i+`valid_bars`.

**Stop-limit on daily bars.** The stop triggers at the open when the open is beyond S. The fill is then the
open if that is within the limit, otherwise the limit if the low (for a buy) reaches it later in the bar. When
the stop triggers inside the bar, the fill is at S if S is within the limit. Otherwise there is **no fill**,
because the bar cannot show whether price came back after touching S.

**Re-entry.** The v1 and Customize rules are unchanged: next-open market mode may share an open, while close
and limit modes give no new order on a bar that exited after its open. A reversal is exempt, because it is one
fill. No signal is taken on the last bar. A position still open on the last bar is marked `open`.

**Resting orders** are not replaced by later signals, and each side has at most one. A new order can be placed
only when no order for that side is resting.

While flat, both sides can rest an order at once; a channel breakout rests a long stop and a short stop. When
only one is touched, it fills, and the other keeps resting as a reversal order. When both are touched on one
bar, nothing fills, in either intrabar mode, because daily bars cannot show which came first.

**Mixed sides** (one side market entries, the other side orders; ruling 2026-10-15, batch 3 review). When both
sides' entries fire on one flat bar and either is a market side, nothing is placed that bar (the ambiguous
rule). When an order rests on one side and the other side's market entry fires, the market entry wins and the
resting order is cancelled. Under `opposite: exit | ignore` a resting order of the other side never acts while a
position is open: it is cancelled when the position opens.

**Delayed exits** (`delay_bars` ≥ 2) belong to the position they were set for. A price exit or a reversal order
that closes that position first cancels the pending exit, and a pending reversal entry whose closing half was
already done this way fills as a plain entry. Pine's re-issued entry (for example a channel breakout whose stop moves each bar) is
`valid_bars: 1`, re-placed every bar.

### 6.3 Same-bar ordering: one default, one cheap option

| Mode | Rule | Who does this |
|---|---|---|
| `stop_first` (**default**, unchanged from Customize) | Levels gapped through at the open fill there first. Then adverse events (the protective level and a reversal stop / stop-limit of the other side) in the order a move from the open reaches them, nearest first (price must cross the nearer level first; a tie takes the protective level; ruling 2026-10-15, batch 3 fix round 2); then favourable events (take-profits, target, a reversal limit) in the same nearest-first order. A stop and a target inside one bar resolve to the stop; no take-profit is booked on a bar whose adverse event fills. Trailing and breakeven levels use bars that have already closed. | AmiBroker fixed stop priority (loss → profit → trailing → N-bar); conservative |
| `ohlc_path` | TradingView's broker-emulator path: if the open is nearer the high, O → H → L → C, otherwise O → L → H → C (a tie takes O → L → H → C). The first level the path reaches fills first: reversal stops / stop-limits sit with the protective level on the adverse leg, reversal limits with the take-profits and target on the favourable leg. Within a leg the level nearest the open fills first, as in `stop_first`. | TradingView Strategy Tester without Bar Magnifier |

A reversal stop-limit is ranked with the adverse events even when its stop triggers at the open and it fills at its
limit later in the bar (for a short: a fill above the open, after the price came back up). This is a declared
simplification: daily bars cannot place that return inside the path, and its fill price is the limit either way.

The option is cheap: it only reorders the candidate events inside one bar. A limit-fill bar still never takes
the target (Customize rule), and a close-fill entry is still not price-checked on its own bar, in both modes.

### 6.4 Signal cleaning

The default `entry_on: level` matches AmiBroker's default backtestRegular. Entry signals that repeat while in
a position are ignored, and the first true bar after an exit enters again, which is `ExRem` behaviour.
`entry_on: edge` requires a false (or unknown) → true transition. A skipped entry is not queued for retry,
again matching backtestRegular.

### 6.5 Equity-level state

`cooldown.after: loss|win`, `risk.max_consecutive_losses` and `risk.max_drawdown_pct` need trade outcomes. Blocked entries also block the opening half of a reversal, which then
acts as an opposite exit: closing is always allowed.
The simulator classifies each closed trade by `Trade::ret(cost)`, using the run's cost per side
(`Compiled.cost`, set by the API before simulating; 0 when no cost is given). Drawdown is on **closed-trade
compounded equity**, evaluated when a trade closes, rather than TradingView's tick-level equity. The response
lists any triggered risk stop in `execution_model.events`.

Rulings (batch 4, 2026-10-15):
- The state reads only trades closed by the signal bar. A next-open exit signalled on the same bar is not counted
  yet, because its fill price is unknown at that close.
- A trade with `ret ≤ 0` counts as a loss for the streak and for `cooldown.after: loss`. This follows the house
  statistics convention, where a zero-return trade is not a win.
- A cooldown counts every exit inside its window, so a later winning trade does not end a loss cooldown (fix round 1).
- A close-fill reversal first realizes its closing trade, then applies drawdown, loss-pause and cooldown rules before opening the opposite side. Risk events are also recorded for realized exits on the final bar; no new signal is taken there.
- A blocked bar cancels resting entry and reversal orders. They are not queued.
- A blocked bar also cancels a delayed market entry (`delay_bars` ≥ 2) that has not filled yet. A pending reversal
  keeps its exit half and becomes a plain exit (fix round 1).
- When a reversal's opening half cannot be sized (`risk_pct` with no stop distance yet), only the exit happens. Its
  reason is "opposite entry", and the event "entry skipped: no stop distance on <date>" is listed.
- `execution_model.events` lists at most 20 events, then "… and N more". Events are reported by `run` only; in
  `optimize` the value is `null`.
- `max_trades` counts positions, not scale-in legs.
- A risk pause or the drawdown stop also blocks scale-in legs. Cooldown and `max_trades` block new positions only.
- `max_trades` counts an entry in the period of its fill, and checks the signal bar's period.
- Scale-in legs are decided at the signal close. The `better` / `worse` filter compares that close with the last
  leg's fill price, and a next-open leg fills whatever its fill bar does. A pending leg belongs to its own position.
  `risk_pct` sizing uses the first leg's stop level, known at the signal bar, and later legs scale from the same base.
  After scale-ins, the stop-loss, target, trailing and breakeven levels stay anchored to the first leg's price. Legs
  on an order side (`entry.limit` / `entry.stop`) fill as market orders.

## 7. Instruments: one explicit execution model per kind

Every run and optimize response gains
`execution_model: {kind, mode, executable, notes[], events[]}`. `mode` is `unit` or `money`.

### 7.1 Cash equity (India and US stocks, US ETFs)

- **Unit mode** (default, unchanged): book unit-allocation returns.
- **Money mode** (`execution.capital` given):
  - Quantities are whole shares, rounded down.
  - Size is computed from equity at the start of the trade and the entry fill price, as AmiBroker does. Scale-in
    legs use the same starting equity. If the fill needs more cash than equity, the quantity shrinks to fit
    (AmiBroker `AllowPositionShrinking`), and a zero quantity skips the trade (counted in `skipped`).
  - A skipped trade's bars are not reused for other signals, because the simulator already counted it as a
    position.
  - No leverage: the notional must be ≤ equity.
- **Shorts, India (stocks and ETFs).** An overnight cash short is not executable in the Indian cash segment:
  a retail short must be squared off the same day unless shares are borrowed through SLB or the position is
  taken in F&O. So:
  - unit mode keeps the v1 result and adds the note "hypothetical — overnight short positions are not
    executable in the Indian cash segment (they need F&O or SLB borrowing)", plus `executable: false`;
  - money mode refuses with `needs_data`: "India cash shorts held overnight need an F&O contract or an SLB
    borrow model; neither is admitted".
- **Shorts, US.** Allowed. Money mode needs a declared `costs.borrow_pct_year` (0 is allowed, but it must be
  stated), charged on notional × calendar days / 365. Unit mode notes "borrow cost not modelled".
- **Corporate actions.** The lab uses the admitted series as the engine already loads it. The basket README
  does not establish its adjustment status, so money-mode quantities are labelled "computed on the admitted
  price series, whose corporate-action adjustment is not established; not verified as-traded quantities".
  This is not fixed here; a follow-up can admit adjustment factors.

### 7.2 Index and spot

- **Index** (India/US indices, gl indices and CFD indices): never tradable. Unit mode stays as v1: a
  **signal study on the index's own levels**, with `executable: false` and the note "an index cannot be
  traded; returns are a study of index levels, not of an ETF or futures contract". Money mode and
  `instrument.kind: futures` on an index ticker answer `needs_data`; the index is never priced as a proxy. A
  proxy trade needs the proxy's own data (an ETF ticker, or a futures chain once admitted).
- **Crypto** (`CRYPTO:*`, a TradingView index composite): a long study only, and also
  `executable: false` (provisional, Phase 5) with the note "CRYPTO:* is a TradingView index composite, not an
  exchange order book". A short gets
  `executable: false` with the note "shorting spot needs a margin borrow or perpetual data with funding, which
  is not staged". Money mode answers `needs_data`, because the series is not an exchange order book.
- **CFD** (`IG:*`, `SPREADEX:*`): unit studies only, provisional (Phase 5). Money mode answers `needs_data`
  until CFD contract terms are admitted.

### 7.3 Futures (contract model; fixtures now, `needs_data` on real requests)

`execution.instrument.kind: "futures"` switches on the model in a new module, `lab/futures.rs`:

- **Contract:** symbol, expiry date, lot size (units per lot), tick size, point value (currency per point per
  unit; 1 for NSE), daily bars (OHLC, volume, OI) and initial margin %. The chain is a list ordered by expiry.
  Lot sizes are **per contract**, because NSE contracts of one underlying can carry different lot sizes at the
  same time.
- **Roll rule** (`instrument.roll`):
  - `{"days_before_expiry":n}` rolls at the close n sessions before expiry; n = 0 means the expiry day.
  - `{"on":"volume"}` / `{"on":"open_interest"}` rolls at the first close where the next contract's volume
    (OI) exceeds the front's.
  - `"none"` means no roll: an open position is force-closed at the expiry close (reason `expiry`).
  - The default is `{"days_before_expiry":1}`.
- **Continuous series for signals** (`instrument.adjust`):
  - `forward` (default): additive. Each later contract is shifted onto the earlier series by old close − new
    close on the roll day. Earlier bars never change when a later roll arrives, so the series is causal.
  - `ratio`: forward multiplicative, by old close / new close.
  - `none`: raw stitched prices, so the jump shows at each roll.
  - `back` (TradingView's B-ADJ) is **refused** as `unsupported`. It shifts every earlier bar at each new roll,
    so a backtest would read prices that exist only after later rolls (lookahead).

  The continuous bar on a roll day is the outgoing contract. Indicators, rules and price levels run on the
  continuous series.
- **P&L on actual contract prices.** Each fill on the continuous series maps exactly to the contract held on
  that bar: actual = (continuous − offset) ÷ factor for that bar's segment. A roll inside a trade splits it into two trades: the old contract is closed at the roll-day
  close and the new contract opened at the same close (`entry_kind: roll`). Both legs pay costs, so roll costs
  are explicit.
- **Orders** are rounded to the contract tick (nearest). `ticks` distances use the contract tick.
- **Sizing.** `units` is in **lots**. `value` and `pct_equity` are sized on initial margin; `risk_pct` uses
  stop points × lot × point value. Every size is rounded down to whole lots.
- **Margin and mark-to-market.** Equity is marked at every close, which is equivalent to daily settlement
  without interest. Initial margin must be ≤ equity at entry, otherwise the quantity shrinks. If equity at a
  close is below the required margin (maintenance = initial, declared), the position is liquidated at the next
  open (reason `margin_call`) and trading stops (`events`).
- **Expiry operands** (§5.2) come from the chain.
- **Limits for now:**
  - futures runs do not combine with scale-in or take-profits;
  - roll `none` force-closes at the expiry close, so it needs `exit_fill: close`;
  - `ticks` distances and tick slippage are futures-only.
- **Real requests.** No futures contract data is admitted (§3.1). Any request with `kind: futures` answers
  400 `needs_data`: "futures contract data for <ticker> (contract chain with expiries, lot sizes and tick size)
  is not admitted; no futures result is published, and the index or spot series is never used as a stand-in".
  The model is exercised only by Rust fixtures (§10). Admitting NSE F&O data is a separate data task (§12).

### 7.4 Position sizing and costs

| Sizing | Unit mode | Money mode |
|---|---|---|
| `pct_equity` p | leg weight p/100 (0 < p ≤ 100) | p% of equity at the trade start ÷ price (cash), or ÷ margin per lot (futures) |
| `risk_pct` r | weight min(1, r / stop distance %) | r% of equity ÷ (stop distance × units per lot × point value) |
| `units` n | — (needs capital) | n shares, or n lots |
| `value` v | — (needs capital) | v currency of notional (cash) or of margin (futures) |

Money-mode costs are all **user-declared**. The engine hard-codes no statutory rate; the response says "user-
declared costs; not a verified statutory schedule".

```json
"costs": { "commission": { "per_order": 20, "per_unit": 0, "pct": 0.03 },
           "slippage": { "bps": 5 } | { "ticks": 1 },
           "taxes": [ { "name": "STT", "pct": 0.1, "side": "both" } ],
           "borrow_pct_year": 0.5 }
```

- Commission is charged on entry and on exit (AmiBroker, TradingView).
- Slippage moves market and stop fills against the trade. Limit fills are not worsened (TradingView rule). Stop-limit entries
  activate limit orders and pay no entry slippage; later scale-in legs remain market fills. Money mode
  currently refuses two-sided resting limit/stop-limit reversals because the simulator retains only
  `entry_kind: reverse`, losing the order kind needed to price both halves' costs. Unit mode and
  money-mode market/stop reversals, or limit sides with `opposite: exit|ignore`, remain supported.
- Each tax is a percentage of fill value on the declared side; there are at most 6 taxes.
- Money mode refuses the bps `cost` query, `slippage` query and `execution.cost_bps`, so costs are never
  counted twice.

**Reusing the book statistics without touching them.** Every money fill gets an *effective* price that
includes that fill's costs per unit: entry is fill + cost/qty for a long, and exit is fill − cost/qty. Each
leg's weight is w = qty × units-per-lot × point value × effective entry ÷ equity at the start of the trade.
Then `Trade::ret(0)` equals the trade's net money P&L ÷ equity at trade start. The book's statistics are
called read-only. A lab-only daily money curve deducts actual entry/exit costs on their fill dates and
accrues short borrow by each leg's calendar days held; the effective-price curve alone cannot correctly
accrue borrow or avoid charging a future exit on an open mark. Gross results use the same whole quantities
and observed prices, without declared costs. Optimized sizing is applied separately for every combination.

Cash money trades pass the existing `trade_validation::check`; futures leverage waits for Task 14's
contract validation. Trade JSON reports observed fill prices, while the separate money ledger reports
whole quantities and each fill's commissions, taxes and slippage, plus accrued borrow and net money P&L.
Effective prices remain internal. Partial exits round down to whole lots, and the final exit closes the
remainder. A marked-open remainder is not an exit order and pays no invented closing commission, tax or
slippage. Quantity sizing uses equity at the trade start; later scale-in legs retain that basis, without
reading later equity or prices. Risk controls still use the simulator's unit returns (assumption 38a),
which can differ from whole-quantity returns after declared costs. The response's money/cost basis names
these limits. Unit-mode responses remain unchanged.

## 8. Compatibility (decided: stay `"v": 1`, additive)

- Every new field is optional, and its default is the v1 / Customize behaviour.
- A spec whose rule lists contain only v1 comparisons compiles through the **unchanged** v1 code path. Each
  top-level v1 comparison becomes the same `RCond` with the same label. Only new node kinds take the new
  evaluator.
- The auth service's `v === 1` check, the saved-strategy storage and the gateway policy need no change.
- Reserved fields made real: `entry.stop` and `entry.scale_in`. The `reserved_fields_are_unsupported` test
  drops its `entry.stop` row. `spec.portfolio`, `spec.scale_in`, `spec.short` and the fill values `limit`/`stop`
  stay `unsupported`.
- **Proof, all in the plan's gate:**
  1. All existing lab tests pass. The only edits are mechanical accessor changes where a test reads
     `spec.entry[k].lhs`, plus the two reserved rows above.
  2. `templates_reproduce_the_book_engine_on_fixtures` passes for all 43 templates.
  3. A new **pinned-output test** records a hash of the trade list of every template on the four fidelity
     fixtures, and of four plain v1 specs, computed at Phase 6 HEAD before any Phase 7 change. It must stay
     identical after every task.
  4. `check_book_untouched.py --engine-compare` reports no problems.
- **Optimize paths.** Every existing path keeps working. New paths are produced by one generic walker:
  - `entry.all[2].any[0].rhs.value`;
  - `entry.all[0].lhs.sum.bars` (whole numbers);
  - `exit.take_profits[1].pct`, `exit.trailing.activate.atr`, `exit.breakeven.after.pct`;
  - `short_side.entry.all[0].lhs.p.length`;
  - `execution.sizing.risk_pct`.

  The grid caps are unchanged: ≤ 1,000 values per range and ≤ 50,000 combinations.

## 9. API changes (engine only, all in `lab/api.rs`)

- `GET lab/catalogue` gains `operators` (extended), `node_kinds`, `expression_kinds`,
  `timeframes: [D,W,M,Q,Y]`, `custom_days: [3,63]`, `expand_modes`, `max_leaves`, `max_nodes`, `max_query_bytes`, and `execution_options` (the enums
  in §6.1). The added keys are additive.
- `GET lab/run` and `GET lab/optimize`:
  - Both gain `execution_model` and, in money mode, `money: {capital, final_equity, costs_total, skipped,
    currency}`.
  - Run trade JSON gains `legs` (scale-in) and `fills` (scale-out pieces) when a trade has more than one. It gains
    `weight` (the position's total unit weight) only when that is not 1. In money
    mode, a `ledger` array gives each trade's entry date, units, position value, costs and money P&L.
  - `entry_kind` gains `stop`, `stop_limit`, `reverse` and `roll`.
- `GET lab/run`, `lab/optimize` and `lab/series` gain a top-level `adaptations` list only when the spec (or
  the series id) reads `sessions_left` (§5.2); otherwise the key is absent and the body is unchanged.
- Error contract unchanged: `invalid`, `unsupported`, `needs_data`. The gateway allowlist and bounds are
  unchanged, and `main.rs` is untouched.

## 10. Testing

### 10.1 No lookahead (every new operand, node and order type)

- **Higher timeframes.** One test for each expand mode and each refused `expandFirst` case (§5.1), plus
  truncation tests on Q, Y and 5D for every computed indicator.
- **Series level.** Every new expression and node kind is evaluated on a 420-bar fixture and on its cuts at
  60, 67, … 393 bars. Every value up to the cut must be identical (the cut bar itself is skipped only for
  W/M/Q/Y/nD values that may show an unfinished period). Only the `sessions_left` indicator (outputs `m`, `q`,
  `y`) is exempt; its own test on a gapped multi-year calendar shows every finite value is unchanged under each
  cut, and a separate test shows that changing prices never changes it.
- **Trade level.** `future_bars_never_change_past_trades` gains a spec for each new feature: two-sided
  reversal, stop entry with `at`, stop-limit, valid_bars 3, delay 2, edge entries, trailing activation,
  breakeven, take-profits, scale-in, cooldown after loss, max_trades, the risk pause, ohlc_path, and Q/Y
  operands. Every bar after *k* is perturbed, and every trade closed by *k* must be unchanged.
- **Futures.** Perturbing contract bars after *k* never changes rolls, fills or trades closed by *k*.
  Forward and ratio offsets never change earlier continuous bars or earlier actual-price fills.

### 10.2 Unit tests per feature

Hand-computed expected trades on 10–30-bar fixtures cover every row of §4.1 and §6.1, three-valued logic, the
limits, the money ledger (rounding, shrink, costs, borrow, effective-price identity: the money P&L equals
`ret(0)` × equity at start to 1e-9), and futures (roll split, adjust modes, ticks, margin call, expiry forced
exit, needs_data on real requests).

### 10.3 Documented-example fixtures (reproduced exactly)

Each fixture writes the published rules as a lab spec, runs it on a crafted OHLC fixture, and checks the trade
list against expected values computed by hand from the **platform's documented semantics**:

| # | Source | Rules | Lab features |
|---|---|---|---|
| E1 | AmiBroker guide, Backtesting ("`Short = Sell; Cover = Buy;`" stop-and-reverse), https://www.amibroker.com/guide/h_backtest.html, with `SetTradeDelays(1,1,1,1)` and `BuyPrice = Open` | MA cross stop-and-reverse | both, reverse, next_open |
| E2 | AmiBroker KB "Handling limit orders in the backtester", https://www.amibroker.com/kb/2014/11/26/handling-limit-orders-in-the-backtester/ | `Buy = cond AND L < limit; BuyPrice = Min(O, limit)` | limit, gap at open |
| E3 | AmiBroker `ApplyStop` (https://www.amibroker.com/guide/afl/applystop.html): `stopTypeNBar` with `stopModeBars` 5, and `stopTypeLoss` with `stopModePercent` and ExitAtStop 1 | N-bar exit, percent stop, gap through stop at open | bars_held, stop_loss |
| E4 | TradingView built-in "Channel BreakOut Strategy" | stop entries at highest(high,5) + tick / lowest(low,5) − tick, reversing | both, always in, stop `at`, two resting orders, reversal orders |
| E5 | TradingView built-in "Consecutive Up/Down Strategy" | 3 rising closes → long, 3 falling closes → short | `at_least` / `count`, reverse at next open |
| E6 | Pine User Manual, Strategies, `strategy.exit` with `profit` / `loss` in ticks and `trail_points` / `trail_offset`, https://www.tradingview.com/pine-script-docs/concepts/strategies/ | bracket and trailing on a futures fixture | ticks, trailing activate, ohlc_path |

The Pine sources of E4 and E5 are TradingView built-ins whose code is visible in the Pine editor. The plan
quotes them from memory, and the implementer must compare the quote with the editor text before committing.
Fixture bars are chosen so that no bar is ambiguous between the two intrabar modes, except where the test
exercises exactly that difference (E6 runs both modes).

### 10.4 Gate

Full engine suite; guard scripts; `check_book_untouched.py --engine-compare`; Customize template fidelity
(the 43 templates on fixtures) plus the pinned-output hash; Phase 5 gl checks (gl run with a both-sides spec
gets the provisional flag; money mode on gl answers needs_data); `scripts/check_upload.py`; a release build
without lab warnings; a bounded real-data smoke on NIFTY 50 (unit mode, needs_data for money and futures); and
a scope proof that the diff touches only `lab/**` and docs.

## 11. Delivery

One backend PR from `Nandini` into `Manan` (never `main`), implemented task by task (subagent-driven, TDD)
from the plan. The user pushes. The rule-builder UI for the new nodes and options belongs to the later
frontend phase.

## 12. Out of scope, named follow-ups

1. Admit NSE F&O futures data: contract chain, lot-size history (`fo_mktlots`), tick, expiry calendar. Run
   it through a `library/market-data`-style admission. Then real futures runs.
2. Options, intraday bars (opening range, intraday square-off for India cash shorts), SLB borrow data,
   perpetual funding data, a dated Indian statutory cost schedule (as an admitted, sourced table).
3. **Phase 8 (planned separately):** a second instrument in rules (Foreign operands such as VIX, TRIN and
   ratios through the reserved `symbol` field), foreign entry and exit, basket scan, ranking, rotation and
   portfolio mode (max open positions, settlement delay).
4. New Customize templates made possible by Phase 7. These are two-sided, scale-in, calendar, "k of n days"
   and "sum of RSI" methods: about 80 of the 121 admitted methods without a template. Each needs its own
   fidelity admission, so they are left to a separate batch.
5. Position-dependent operands inside groups (`entry_price`, open profit), volatile ATR stops (AmiBroker
   `volatile=True`), and the AmiBroker same-bar `AllowSameBarExit` matrix as an option.
6. (Review follow-up d.) Compute `sessions_left` from a scheduled per-exchange holiday calendar once one is
   admitted, instead of the data's own session dates.

## 13. Assumptions for the user's review

Each item was decided without the user, who was unavailable. Each gives the reason and the cost if it is
wrong.

1. **Stay on `"v": 1`; additive fields only.** *Reason:* every saved spec and template keeps its meaning; the
   auth service and gateway need no change. *Cost if wrong:* a later v2 would need a converter; nothing breaks
   now.
2. **A node tree, not a free-text formula.** Nodes cover AFL/Pine conditions on one instrument; no user code
   runs. *Reason:* spec §1 is form-based, and a tree is validated field by field. *Cost:* very long AFL
   expressions need several nodes; the 40-leaf and 120-node caps may bind for exotic systems.
3. **Three-valued logic, with unknown = false at the top.** *Reason:* a `not` must never fire during warm-up.
   *Cost:* a user who expects Pine's `not na`-style behaviour sees fewer early signals.
4. **`bars_held` stays a top-level exit operand.** *Reason:* nodes are precomputed arrays, and position state
   would force per-bar re-evaluation. *Cost:* "bars_held ≥ 5 AND rsi > 70" must be written as a sequence or as
   two exits (or waits for follow-up 5).
5. **`rising` means strictly monotone.** The Pine meaning is shown as an expression. *Reason:* it is the plain
   reading of "rising for N bars". *Cost:* users of Pine's `ta.rising` must use the expression form.
6. **Q and Y are calendar quarters and years**, usable after the period's last session closes. *Cost:*
   fiscal-year anchors (for example India April–March) are not offered.
6a. **AmiBroker higher-timeframe semantics are adopted with lookahead-free modes only.** `expand:"last"` is
    the default and equals v1. `point` is always allowed. `first` is allowed only for completed earlier
    periods (offset ≥ 1), the period open, or `compress` mode `first`. *Reason:* the user asked for
    AmiBroker's TimeFrame behaviour; AmiBroker's own docs warn that expandFirst looks ahead. *Cost:* an AFL
    system that relies on expandFirst of a high/low/close at offset 0 is refused rather than reproduced.
6b. **N-day custom periods (`2D`…`63D`) count sessions from the instrument's first admitted bar.** *Reason:*
    deterministic, and independent of the chosen window. *Cost:* AmiBroker's own N-day anchoring is not
    documented and may differ, so block boundaries can disagree with an AmiBroker chart.
6c. **`"symbol"` on an operand is reserved for Phase 8** (multi-symbol) and answers `unsupported` now.
    *Cost:* none; it is the seam.
7. **`sessions_left` (outputs `m`, `q`, `y`) uses future session dates, never prices.** *Reason:* the published exchange calendar
   is known in advance; this is the only way to say "last N trading days" exactly. *Cost:* the data's session
   dates also encode unscheduled closures, late-announced closures or special sessions, and data gaps, none of
   which a live trader knew in advance, so a backtest signal can differ from live; every response that reads
   it carries an `adaptations` note, and `session.weekdays_left_*` is the strictly causal alternative.
8. **Expiry operands exist only inside futures runs.** *Cost:* none until futures data is admitted.
9. **Two-sided specs keep `entry`/`exit` for the long side and add `short_side`.** *Reason:* v1 meaning is
   untouched and the shape mirrors the long side. *Cost:* the UI must label the top-level lists "long" when
   `direction` is `both`.
10. **Both sides' entries true while flat → no entry.** *Reason:* symmetric and conservative; AmiBroker's
    "Buy wins" is arbitrary and Pine's depends on code order. *Cost:* an AB system relying on Buy-wins differs
    on those bars.
11. **`opposite: reverse` is the default for `both`.** *Reason:* it is both platforms' default. *Cost:* none;
    `exit` and `ignore` exist.
12. **At most one position change inside a bar, except a reversal fill.** *Reason:* daily bars do not show
    the sequence. *Cost:* a stop-out followed by a new entry order on the same bar's adverse leg is missed
    (TradingView would fill it).
13. **Caps: 8 KB spec and 12 top-level items unchanged; 40 leaves and 120 nodes new.** *Reason:* the gateway
    is additive-only and already enforces the first two; the new caps bound evaluation cost (each node is one
    O(n) array). *Cost:* raising them later is a constant change in two files.
14. **GET stays.** POST for engine routes would change the gateway's core request handling. *Cost:* a spec
    that URL-encodes past 16 KB is refused; the catalogue publishes the limit so the UI can warn.
15. **The gateway is not changed.** The engine is the authority for leaves, nodes and nesting. *Cost:* a
    malformed large spec reaches the engine before being refused (bounded by the 8 KB cap).
16. **Default intrabar rule stays `stop_first`; `ohlc_path` (TradingView) is an option.** *Reason:* stop_first
    is the Customize behaviour and AmiBroker's documented priority; ohlc_path is a cheap reorder. *Cost:*
    results differ from TradingView by default on bars where stop and target are both inside the range.
17. **Stop-limit with an intrabar trigger and the limit beyond the stop → no fill.** *Reason:* path unknown.
    *Cost:* a few fills missed against TradingView.
18. **Resting orders are not replaced while they rest.** *Reason:* simple and causal; per-bar re-placement is
    `valid_bars: 1`. *Cost:* a GTC order whose level should trail needs `valid_bars: 1`.
19. **`delay_bars` applies to next-open market orders only.** *Cost:* AmiBroker delays on close fills are not
    offered (they equal a next-open entry with a different price).
20. **`min_bars` blocks rule exits only; stops stay active.** *Reason:* ignoring a placed stop overstates
    safety. AmiBroker's HoldMinBars blocks all exits. *Cost:* an AB HoldMinBars system differs if a stop fires
    early.
21. **Scale-in cannot combine with take-profits.** *Reason:* the book trade ledger defines scale-out
    fractions of one position. *Cost:* that combination is refused (invalid) for now.
22. **Take-profit fractions are of the initial position** (Pine `qty_percent` since 2022). *Cost:* none.
23. **Equity-level rules use closed-trade compounded equity at each exit.** *Reason:* causal and simple.
    *Cost:* TradingView's tick-level `max_drawdown` can stop earlier.
24. **Unit mode stays the default; money mode starts when `execution.capital` is given.** *Reason:* v1
    results and the book statistics stay as they are; money mode reuses them through effective prices.
    *Cost:* money and unit results are not comparable, so the response labels the mode.
25. **Money sizing uses equity at the trade's start and the entry fill price; it shrinks to fit, and 0 skips.**
    *Reason:* AmiBroker's sizing and shrinking; the fill price is known when the size is set. *Cost:* the result
    differs slightly from TradingView, which sizes on the signal close.
26. **No leverage for cash; futures margin is the user's declared % (maintenance = initial).** *Cost:* real
    exchange SPAN margins and maintenance below initial are not modelled.
27. **Costs are user-declared lists; no statutory rate is hard-coded.** *Reason:* AGENTS.md (no unsourced
    statutory rates) and dated schedules are not admitted. *Cost:* users must enter STT and other charges
    themselves.
28. **Slippage worsens market and stop fills, not limits** (TradingView). *Cost:* none.
29. **India cash overnight shorts:** unit mode labelled "hypothetical", money mode refused. *Reason:* v1
    results keep their meaning, while money mode (the executable claim) refuses what the cash segment cannot
    do. *Cost:* a user wanting a "what if SLB" estimate cannot get money figures.
30. **US shorts need a declared `borrow_pct_year` in money mode.** *Cost:* one extra field.
31. **An index is a signal study only; money and futures modes on an index answer `needs_data`.** *Cost:*
    none for v1 (unit studies unchanged).
32. **Crypto and CFD money mode answer `needs_data`.** *Reason:* TradingView composite/CFD quotes are not
    exchange order books, and their contract terms are not admitted. *Cost:* crypto money results wait for
    licensed spot data.
33. **Futures: forward-adjusted signals by default, backward adjustment refused (lookahead); P&L on actual
    contract prices; roll at the close one session before expiry by default; tick rounding to the nearest
    tick.** *Cost:* a TradingView B-ADJ chart's historical levels differ from the lab's forward-adjusted ones.
    Users who expect TradingView's unadjusted `1!` must set `adjust: none`.
34. **Futures margin call liquidates at the next open and stops trading.** *Reason:* a deterministic end
    state; TradingView's partial 4× liquidation is not reproduced. *Cost:* a run after a margin call shows no
    further trades.
35. **Futures answer `needs_data` on every real request.** *Reason:* no admitted chain, lot, tick or expiry
    data (§3.1); the share's FO bhavcopies are one month without lot sizes; stitched series lack contract
    identity. *Cost:* the user's futures goal waits for follow-up 1; the model is ready and tested.
36. **Money-mode quantities carry a corporate-action caveat** rather than a refusal. *Cost:* if the admitted
    series are not adjusted, share counts around splits are approximate; the label says so.
37. **No new templates in this phase.** *Reason:* each template needs a fidelity admission against the book
    engine, which is its own batch of work. *Cost:* Customize coverage stays at 43 until that batch runs.
38a. **Two resting orders touched on one bar fill nothing; blocked entries turn a reversal into an exit;
    scale-in legs fill at market even when the first entry was an order; stop levels come from the first leg;
    equity-level rules read unit returns (also in money mode).** *Reason:* each is the simplest causal rule.
    *Cost:* rare bars and money-mode risk rules differ from a full money-aware engine.
38. **Documented-example fixtures are hand-computed from the published semantics**, not exported from
    AmiBroker or TradingView (no install or export here). The Pine built-in sources are quoted from memory and
    must be checked in the Pine editor. *Cost:* if a quote differs, that fixture's spec changes; the engine
    semantics do not.

## 14. Sources

- AmiBroker: Multiple time frame support https://www.amibroker.com/guide/h_timeframe.html; TimeFrameSet
  https://www.amibroker.com/guide/afl/timeframeset.html; TimeFrameCompress https://www.amibroker.com/guide/afl/timeframecompress.html;
  TimeFrameExpand https://www.amibroker.com/guide/afl/timeframeexpand.html; TimeFrameGetPrice
  https://www.amibroker.com/guide/afl/timeframegetprice.html; ApplyStop https://www.amibroker.com/guide/afl/applystop.html; Backtesting
  https://www.amibroker.com/guide/h_backtest.html; Portfolio backtester (same-bar scenarios, ReverseSignalForcesExit)
  https://www.amibroker.com/guide/h_portfolio.html; SetPositionSize https://www.amibroker.com/guide/afl/setpositionsize.html;
  SetOption https://www.amibroker.com/guide/afl/setoption.html; SetTradeDelays https://www.amibroker.com/guide/afl/settradedelays.html;
  ExRem https://www.amibroker.com/guide/afl/exrem.html; Futures mode https://www.amibroker.com/guide/h_futbacktest.html;
  Pyramiding / scaling https://www.amibroker.com/guide/h_pyramid.html; Time frames https://www.amibroker.com/guide/h_timeframe.html;
  KB limit orders https://www.amibroker.com/kb/2014/11/26/handling-limit-orders-in-the-backtester/; KB stop priority
  https://www.amibroker.com/kb/2006/09/29/stops-priority-in-the-default-backtest-procedure-in-amibroker/; KB risk-based sizing
  https://www.amibroker.com/kb/2014/10/12/position-sizing-based-on-risk/; forum gap-through-stop fills at the open
  https://forum.amibroker.com/t/39477.
- TradingView: Strategies https://www.tradingview.com/pine-script-docs/concepts/strategies/ (broker emulator O-H-L-C path,
  gaps fill at the open, stop-limit, OCA, pyramiding, reversals, trailing `trail_points`/`trail_offset`, `qty_percent` of the
  initial size, `strategy.risk.*`); Pine v6 reference https://www.tradingview.com/pine-script-reference/v6/; Strategy
  properties https://www.tradingview.com/support/solutions/43000628599-strategy-properties/; Broker emulator
  https://www.tradingview.com/support/solutions/43000786181-broker-emulator/; percent-of-equity sizing
  https://www.tradingview.com/support/solutions/43000651235; continuous futures roll and back-adjustment
  https://www.tradingview.com/support/solutions/43000689313, https://www.tradingview.com/support/solutions/43000685266;
  repainting / `request.security` lookahead https://www.tradingview.com/pine-script-docs/concepts/repainting/.
- India market rules (short selling, lot sizes, expiry): SEBI master circular on short selling
  SEBI/HO/MRD/MRD-PoD-3/P/CIR/2024/1 (5 Jan 2024); SEBI SEBI/HO/MRD/TPD-1/P/CIR/2024/132 (1 Oct 2024, index derivative
  contract size); NSE circular NSE/FAOP/70616 (3 Oct 2025, lot revision). Used for the execution-model labels only; no rate
  from them is hard-coded.
- Project research (2026-10-14, outside Git): the AmiBroker backtester, TradingView broker emulator and India differences
  catalogues. Their findings are cited above by original URL.
