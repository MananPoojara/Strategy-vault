# Strategy Vault

Trading-book research library and backtesting application, with a React + TypeScript + Vite interface, Rust calculation engines and axum gateway, Better Auth, PostgreSQL and Docker Compose/Caddy.

Application code and research tooling are included. Market datasets, original books, database contents, secrets, generated workbooks and private conversations are deliberately external. No sample performance is substituted for missing data.

## Collaboration

All Manan development goes on **`Manan`**. Each other developer creates a separate branch. **No direct commits or pushes to `main`.**

```bash
git clone --branch Manan https://github.com/MananPoojara/Strategy-vault.git
cd Strategy-vault
./scripts/install-hooks.sh
# Other developer:
git switch -c your-name/page-name
```

Nandini (collaborator, write access) works on **`Nandini`**, kept up to date by merging `origin/Manan`; her work reaches `Manan` only through a reviewed pull request.

The repository was empty when imported. The initial code is published only on `Manan`; no `main` branch or main commit is created by this import. If main is introduced later, integrate reviewed changes through pull requests.

## Frontend

Requirements: Node.js 22.12+ (the Docker build uses24.11.1), npm and access to an authenticated application backend.

```bash
cd Context/guru/frontend/react
npm ci
npm run dev
npm run build
npm run test:coverage
```

Vite uses a same-origin development proxy to `http://127.0.0.1:8180`. Set `VAULT_DEV_PROXY` for a different backend. The backend must allow your development origin for sign-in/settings/write requests; do not bypass origin checks. Prefer a local backend with local private runtime configuration. See [developer setup](docs/DEVELOPMENT.md).

Register new React pages in `src/app/custom-pages.ts`. Pages share the existing layout, account controls and styles. See [adding a page](docs/ADDING-A-PAGE.md).

## Full application

See [runtime data setup](docs/DATA-SETUP.md) and the existing [webapp instructions](Context/guru/webapp/README.md). Supply the real data outside Git, then run:

```bash
VAULT_SOURCE_ROOT=/absolute/path/to/private-source-workspace python3 Context/guru/webapp/scripts/setup.py
docker compose -f Context/guru/webapp/compose.yaml --env-file "$HOME/.cache/strategy-vault-webapp/.env" up -d --build
```

The setup script preserves existing secrets and generates private credentials on a fresh setup. It does not discover or import the whole research NAS. Never commit its output.

## Strategy status

The complete source-rule audit is **unfinished**. Current available result counts are documented in [strategy review status](docs/STRATEGY-STATUS.md). Registered methods, parameter variants and enabled results do not certify complete book-faithful conversions or reproduction of published studies.

## Indicators lab

User-defined indicator strategies (`#/indicators`), developed on branch **`Nandini`**. Every result is labelled *simulated · not a book method*; book strategy code and results are never changed by lab work (`python3 scripts/check_book_untouched.py --base origin/Manan` must report no problems).

- **Indicator library:** 606 indicators by category (492 computed, the rest listed with the data they need), collapsed by default. They come from TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock and TA-Lib, with formulas checked against published sources. Platform formula variants (e.g. MetaTrader ADX/ATR) are a `variant` setting whose default keeps today's values.
- **AmiBroker-style analysis:** Parameters window per chart chip; Analysis settings tabs (General · Trades · Orders & stops · Portfolio) with trade prices/delays, stop exit modes, re-entry delays, position limits, tick size, margin, pad & align, risk-free rates; Backtest, Scan, Explore, Optimize, Individual optimize, 3D optimization chart, Walk-forward and Monte Carlo. Results open as tabs under *Run backtest*: Performance metrics (All / Long / Short / Buy&Hold, Formula and Symbols pages) · Equity & drawdown · Monthly returns · Trades · Monte Carlo · Scan / Explore. Feature-by-feature status: [AmiBroker parity gaps](docs/superpowers/specs/2026-10-16-indicators-lab-amibroker-parity-gaps.md).
- **Custom indicators:** an AmiBroker-like formula (`Param`, `Plot`, arithmetic, `x[n]`, `IIf`, `Cross`, MA/EMA/HHV/LLV…, any built-in indicator by id) or a drag-and-drop block builder; saved per user under *My indicators*, charted, usable in rules and optimizable. Design: [custom indicators](docs/superpowers/specs/2026-10-19-custom-indicators-design.md).

## Tests

```bash
# Engine (Rust) — VAULT_IDX points at the private idx_data.csv
cd Context/guru/backend && VAULT_IDX=/path/to/idx_data.csv cargo test --release
# Frontend
cd Context/guru/frontend/react && npx tsc --noEmit -p . && npx vitest run
# Gateway + Node unit tests + browser/data checks
Context/guru/webapp/scripts/verify.sh
```

Integration tests run only against an **isolated dev stack** (`strategy-vault-dev-<name>`, own database and port, never port 8180) — see [two developers on one machine](docs/DEVELOPMENT.md):

```bash
cd Context/guru/webapp
export VAULT_ENV_FILE=$HOME/.cache/strategy-vault-dev-<name>/.env VAULT_COMPOSE_PROJECT=strategy-vault-dev-<name>
node --test tests/integration.test.mjs tests/lab-strategies.integration.test.mjs tests/lab-indicators.integration.test.mjs
node --import ./tests/ts-loader.mjs --test tests/library-context-integration.test.mjs   # imports frontend .ts files
python3 tests/data_readiness.py                                                          # needs Python Playwright + Chrome
VAULT_RUNTIME_ROOT=$HOME/.cache/strategy-vault-dev-<name> python3 tests/chart_data_audit.py
```

Sign-in is rate limited; when running several suites back to back, restart the dev stack's `auth` service between them. `verification/reference/` (captured legacy responses) is git-ignored and exists only on the reference machine; without it those comparisons are skipped with a message.

## React migration scope

React owns the production layout, navigation, route selection, library home, cards, search/filter controls, source reading pages and Saved page. Complex author/strategy/optimization presenters and chart/export helpers remain in an explicit compatibility module to preserve reviewed behaviour. Their HTML strings are converted into React nodes; inline scripts/unknown handlers are discarded, and no iframe or full-page `innerHTML` wrapper is used. New development uses React components. A complete rewrite of every existing widget into idiomatic TSX remains separate work; see [migration notes](docs/REACT-MIGRATION.md).
