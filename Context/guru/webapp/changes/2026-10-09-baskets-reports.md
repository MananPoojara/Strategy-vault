# Basket, cost and report changes

User source: four screenshots and the 2026-10-09 request. Native DATABASE source is read-only; imports belong to project/private cache and must preserve source bytes and timestamps. User clarification: no capital-related statistics or assumed portfolio allocation. Average net P&L is a closed-trade return percentage; slippage bps per side; rollover bps per actual roll.

- Basket choices NSE 500, Nifty 50, Midcap100, All F&O beside variants, after period controls; constituent symbols rather than relabeled index charts. Independently tested symbols; combined portfolio is not defined.
- Remove Rust timing/bar/trade footer; retain main fonts/cards.
- Add explicit slippage and rollover settings to shared run/optimization requests; no synthetic rolls from expiry guesses.
- Seven primary metrics only: Net Profit, Average Net P&L, No. Trades, Win Rate, Expectancy, MaxDD, Sharpe.
- Equity date-range handles and year shortcuts; chart image export.
- Monthly table color choices, filled Avg/Total corner, true zero versus absent months.
- Whole-page persisted themes, expanded comparison against actual market benchmark.
- Collapsed detailed statistics before monthly returns, no repeated primary or capital figures; gross/net definition explicit.
- Exact requested 17-column tradesheet and styled Excel; unavailable monetary/tax/contract/excursion inputs stay unavailable.
- Share local page link, durable tags and administrator-controlled CEO recommendation, PDF export at top. Share click does not send messages automatically.
- Real browser/API/export checks on deployed LAN; bounded workers and existing engine memory/CPU limits.

Implemented in the authenticated LAN app. The main interface is `../frontend/full-variant1.html`, with `report-data.js`, `report-ui.js` and `report-controls.css`; optimization uses `optim.js`. Execution accounting and lazy native-series admission are in `../backend/src/accounting.rs` and `baskets.rs`. Tags/themes use `auth/src/`, `db/001-app.sql` and existing gateway app routing. Deployment uses a separate read-only `/basket-data` bind.

Coverage: NSE 500 499/500 (empty GSPL missing); Nifty 50 50/50; Midcap100 100/100; user-selected All F&O cash 224/224. The detailed import audit is `../../../../library/market-data/README.md`. Selected source files retain their hashes and modification times. Invalid historical OHLC prefixes were quarantined unchanged; derived files use only disclosed contiguous valid suffixes, with requests before supported coverage withheld. Current membership is not historical membership; Midcap uses a 2026-10-09 official snapshot and the latest quotes are 2026-10-01.

Remaining inputs: verified futures contract/roll mapping for positive rollover; genuine option data/accounting for unavailable option strategies; benchmark volume for affected stock filters; company bull/bear patch mapping; monetary quantities/tax schedule only if such amounts are requested later. No capital statistics or allocation were added. Exact intraday MAE/MFE remains unknown; displayed observations are labeled partial gross daily-bar bounds. Page PDF uses the native browser Print / Save as PDF workflow.

Actual API/browser/export evidence and the tested image versions are recorded in `../verification/RESULTS.md`.
