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

## Lead and handoff

- Codex is the active lead as of 14 November 2026, until the user names another lead. Only the active lead integrates and pushes; collaborators use separate worktrees and do not edit the lead's working tree.
- At the start of a session, read `CODEX-CONTINUE.md` and the `RESUME HERE` section of `HANDOFF.md` in the private project workspace. Recheck their dated status against the checkout and runtime. Before stopping or at a usage limit, update that handoff section and append a dated log line without removing history.
- Integrate in a separate worktree and fast-forward the authoritative `Manan` checkout after review; do not switch its branch. Run bounded builds and verification as described in the handoff.
- Do not change calculations or deploy without the user's approval. Compare before and after results for affected existing strategies. Cite source pages, obtain an independent check, and label adaptations and unresolved data honestly.
- Preserve old runs and datasets unless the user authorizes deletion. Check available disk and memory before heavy work; do not assume old status counts or tmux sessions are current.

- User preference (14 November 2026): run subagents with `gpt-6.1-terra` and `model_reasoning_effort="high"`; use the tmux `vault:wall` grid for visible lanes, keeping source reviews in separate worktrees and heavy jobs under `authors/heavy`.
