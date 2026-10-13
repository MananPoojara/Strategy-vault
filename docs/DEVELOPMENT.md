# Developer setup

## Components

- `Context/guru/frontend/react`: active React/TypeScript/Vite application.
- `Context/guru/backend`: Rust strategy engine.
- `Context/guru/webapp/gateway`: authenticated Rust/axum gateway.
- `Context/guru/webapp/auth`: Better Auth/Hono service.
- `Context/guru/webapp/db`: schema migrations.
- `Context/guru/webapp/compose.yaml`: PostgreSQL, auth, both engines, gateway and Caddy.
- `library`: code for inventory/import/source checks; original PDFs and generated inventories are external.

The frontend build is part of the gateway Docker image. It uses a committed npm lockfile. `npm run prepare:assets` copies existing licensed/local image/font/chart/export assets into the generated public directory. No datasets are copied into the frontend build.

## Local runtime

Follow DATA-SETUP.md. Install Node 22.12+ and Docker Compose. `setup.py` stages only configured real inputs and writes secrets on local disk. Its output stays outside Git. Missing inputs are reported; there is no synthetic market-data fallback.

For frontend development, run the local backend on8180 and Vite on5173. Add `http://127.0.0.1:5173,http://localhost:5173` to the local runtime's `VAULT_ALLOWED_ORIGINS` and recreate auth/gateway for sign-in and writes through the proxy. Retain `VAULT_PUBLIC_URL` matching the local application URL. Do not edit the shared live deployment's credentials or origins for another developer.

A backend with no admitted reading PDFs can show indexed metadata, but the reader must report missing copies. Historical constituent membership and corporate-action adjustment assumptions remain explicitly unverified where applicable.

## Checks

```bash
npm --prefix Context/guru/frontend/react ci
npm --prefix Context/guru/frontend/react run build
npm --prefix Context/guru/frontend/react run test:coverage
python3 scripts/check_upload.py
```

Use the offline regression runner `scripts/test_frontend_helpers.mjs` for preserved helpers. Actual-LAN browser tests require private runtime data/account settings and are not automatic CI fixtures. Source calculations have existing Rust tests; run with bounded Cargo jobs when those calculations change.

Each checkout installs the Git hooks itself. Local hooks can be bypassed, so a repository main ruleset supplements them. No main commits are needed for the initial Manan-only publication.

## Two developers on one machine

Use separate Git checkouts and branches. Do not start or recreate the shared live Compose project while testing your branch. For an isolated backend, use a separate runtime directory, a separate port, a separate engine image tag and a separate Compose project name:

```bash
VAULT_RUNTIME_ROOT="$HOME/.cache/strategy-vault-dev-yourname" VAULT_SOURCE_ROOT=/absolute/private/source python3 Context/guru/webapp/scripts/setup.py
# In that private runtime .env, choose another VAULT_PORT/public URL and add:
# VAULT_ENGINE_IMAGE=strategy-vault-engine-dev-yourname:local
docker compose -p strategy-vault-dev-yourname -f Context/guru/webapp/compose.yaml --env-file "$HOME/.cache/strategy-vault-dev-yourname/.env" up -d --build
```

A separate Compose project also creates its own database volume. Configure the corresponding public URL/origins consistently. Keep the established shared live deployment intact.
