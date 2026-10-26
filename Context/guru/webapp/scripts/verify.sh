#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
node --experimental-test-coverage --test tests/contracts.test.mjs tests/backtest-trust.test.mjs tests/research-review.test.mjs tests/optimization-export.test.mjs tests/analyst-conclusion.test.mjs tests/conclusion-presentation.test.mjs tests/report-data.test.mjs tests/legacy-response.test.mjs tests/research-portal.test.mjs
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/test-target" nice -n 10 cargo test --locked --manifest-path gateway/Cargo.toml
node --test tests/integration.test.mjs
python3 tests/chart_data_audit.py
python3 tests/data_readiness.py
python3 tests/joe_ross_ceo.py
python3 tests/analyst_browser.py
python3 tests/reports_browser.py
python3 tests/browser_smoke.py
python3 tests/login_design.py
node --test tests/persistence.test.mjs
