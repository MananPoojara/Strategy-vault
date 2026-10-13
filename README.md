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

## React migration scope

React owns the production layout, navigation, route selection, library home, cards, search/filter controls, source reading pages and Saved page. Complex author/strategy/optimization presenters and chart/export helpers remain in an explicit compatibility module to preserve reviewed behaviour. Their HTML strings are converted into React nodes; inline scripts/unknown handlers are discarded, and no iframe or full-page `innerHTML` wrapper is used. New development uses React components. A complete rewrite of every existing widget into idiomatic TSX remains separate work; see [migration notes](docs/REACT-MIGRATION.md).
