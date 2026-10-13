# Strategy Vault application

The active frontend is [React + TypeScript + Vite](frontend/react). The historical HTML presentations are retained as reference, while the gateway serves the compiled React application.

- `backend/`: Rust calculation engine and strategy admission rules.
- `frontend/react/`: active application and new-page registration.
- `webapp/`: authentication, gateway, database schema, deployment and existing verification tools.
- `tools/`: research/verification utilities.

Start with the root README and `docs/DEVELOPMENT.md`. Real data, original books, secrets and generated outputs remain external. All Manan changes use the `Manan` branch; no direct main commits/pushes.
