# Strategy Vault collaboration

- Work on `Manan` for Manan's changes, or a separate branch for another developer.
- Never commit or push directly to `main`. Never force-push a shared branch.
- Run `scripts/install-hooks.sh` on each checkout.
- The active frontend is `Context/guru/frontend/react`; add pages in JSX/TSX and register them in `src/app/custom-pages.ts`.
- Historical HTML presentations are retained as reference; they are not the production entry point.
- Preserve existing design, account roles, source-review notices and instrument admission rules.
- Do not publish results where the required rules or data are unresolved. A catalogue registration is not verification.
- Stock methods require actual stocks. An index cannot substitute for a stock universe, and a cash index cannot substitute for ETF/options/futures prices.
- Raw market data, PDFs, database contents, local `.env`, generated result files and private conversations stay outside Git. Runtime datasets mount read-only.
- Do not write to the original research or market-data shares. Keep build/test concurrency bounded; no whole-corpus backtest at startup.
- Verify frontend changes with type checking, the React tests and appropriate real-browser checks. Do not change Rust strategy calculations as part of presentation work.
- Keep verified source rules, declared adaptations, simulated results and book-reported figures clearly separate.
