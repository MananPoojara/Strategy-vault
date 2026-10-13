#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
backup_dir="${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/backups"
mkdir -p "$backup_dir"
chmod 700 "$backup_dir"
backup="$backup_dir/vault-$(date -u +%Y%m%dT%H%M%SZ).sql"
umask 077
docker compose --env-file "${VAULT_RUNTIME_ROOT:-$HOME/.cache/strategy-vault-webapp}/.env" exec -T postgres pg_dump --clean --if-exists -U vault -d vault > "$backup"
echo "Database backup saved to $backup"
