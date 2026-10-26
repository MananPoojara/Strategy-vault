# Timeless charts in the Indicators lab (design, 2026-10-24)

Research catalogue (algorithms, citations, open items): `/tmp/claude-1000/-run-user-1000-gvfs-smb-share-server-192-168-4-50-share-share-Manan-Pujara-Book-to-rust/36e86354-3ce2-4264-b600-d9a242d6046f/scratchpad/timeless/CATALOGUE.md` and the notes files beside it.

## Intent
Users can use every timeless (price-only) chart type that can be built from daily OHLC data in two ways: as normal lab indicators in Entry, Filter and Exit rules (backtest and optimize like Supertrend or MFI), and as a chart view drawn without a date axis. Results must be explainable and match a named platform.

## Decisions (user)
- All buildable timeless types, both chart and rules.
- Where platforms differ, the user picks the platform by name in a dropdown ("AmiBroker", "TradingView", "StockCharts", "Sierra Chart", "MetaStock"). Never invent internal or mode names in the UI. The default is the AmiBroker behaviour when one is known, otherwise the most common one (cite the source in the definition text).
- Existing results never change: book code is untouched (`scripts/check_book_untouched.py`), and existing lab indicator outputs keep their values (new outputs and params are additive, with defaults equal to today's behaviour).

## Scope
Build:
1. **Renko** (extend the existing `renko` in place, additive): new outputs `brick_high`, `brick_low`, `flip`, `run`, `confirm2`. Box modes are ATR at the bar (existing), fixed points (existing), and % of that bar's close. Options: grid-aligned anchor; source Close (default) or High/Low (continuation extreme first, a declared rule). Wicks only if they fall out naturally.
2. **Line Break** (`line_break`): N lines (default 3). Outputs line open/close, direction, new_line, flip, run, rev_level, cont_level. The reversal rule when fewer than N lines exist is a platform dropdown (AmiBroker/MetaStock default, Sierra Chart, TradingView if confirmed).
3. **Point & Figure** (`pnf`): source Close or High/Low; box fixed / % / ATR at the bar; reversal 1 to 5 (default 3); one-step-back. Outputs per CATALOGUE §2.3 (col_dir, col_high, col_low, boxes, new_col, boxes_added, rev_level, buy_sig, sell_sig, signal). Pattern outputs (separate id `pnf_patterns`, or extra outputs) only for the patterns the catalogue marks well-defined (double/triple/quadruple top/bottom breakout, ascending/descending triple, triangles, bull/bear trap, long tail, poles, vertical-count objective). Patterns that need a user choice are left out and listed in the definition text.
4. **Kagi** (`kagi`): reversal by points / % / ATR at the bar. Outputs: line level, direction, yang/yin (thickness), new reversal, shoulder/waist levels, yang/yin change event.
5. **Heikin Ashi**: already an indicator. It is offered as a chart view only.
6. **Intraday-only types** (range, reversal, momentum, Kase, Flex/Inverse Renko, volume/tick/delta bars): listed in the catalogue as needs-intraday entries (not computed), the same way the existing intraday indicators are listed.
7. **Chart view** (frontend): a chart-type switch (Candles, Heikin Ashi, Renko, Point & Figure, Kagi, Line Break) that draws bricks, X/O columns, Kagi lines or line-break boxes on a non-date axis. These are rebuilt from the engine's per-bar outputs (no new API unless unavoidable). Hover shows the dates. Trade entry/exit markers sit at real prices on the brick or column that contains that date. Parameters come from the same Parameters dialog as the indicator.

## Rules for every lab output
- Aligned 1:1 with the daily bars and causal: a value uses only bars up to and including that bar (the lab's causality test must pass). The look-ahead box sizing that TradingView and StockCharts apply to the whole history (last-price %, last ATR) is not offered.
- Events are 1/0 (or ±1/0). Levels are prices. NaN before the first brick, line or column.
- Definition text states the exact algorithm, defaults and the platform followed, with a citation.
- Register like any other indicator (catalogue, sources attribution, tests, parity-fixture docs where used), and follow the existing code style.

## Verification
- Unit tests per type with hand-computed cases.
- An independent re-implementation, written from the catalogue rather than from the code, checked against the engine on NIFTY 50, NIFTYBANK and NIFTYMIDCAP100 daily data, per type and per main option.
- Full engine and frontend test suites, a type check, and the book-untouched guard.
- Deploy to the dev server (192.168.4.34) after the user approves, then check in Chrome.

## Left to the builder
Output naming within the catalogue proposals, parameter ranges and caps (for example a brick-count cap for the chart view), and P&F glyph styling (X/O).
