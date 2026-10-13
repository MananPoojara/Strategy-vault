#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
python3 scripts/setup.py
for service in auth engine-in gateway; do
  docker compose --env-file "${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/.env" build "$service"
done
docker compose --env-file "${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/.env" up -d --wait --wait-timeout 180
