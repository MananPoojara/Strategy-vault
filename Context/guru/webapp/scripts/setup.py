#!/usr/bin/env python3
"""Prepare only the selected datasets and generate secrets without overwriting them."""
from pathlib import Path
import os, secrets, shutil
app = Path(__file__).resolve().parents[1]
root = Path(os.environ.get('VAULT_SOURCE_ROOT', str(app.parents[2])))
runtime = Path(os.environ.get('VAULT_RUNTIME_ROOT', str(Path.home()/'.cache/strategy-vault-webapp')))
data = runtime/'data'
data.mkdir(parents=True, exist_ok=True)
for source, target in [
    (root/'idx_data.csv',data/'idx_data.csv'),
    (root/'india_vix',data/'india_vix'),
    (root/'extra_data/india',data/'extra_data/india'),
    (root/'extra_data/us',data/'extra_data/us'),
    (root/'verification/us-book-replication/us_data.csv',data/'us_data.csv'),
    (root/'verification/us-book-replication/vix',data/'us_vix'),
]:
    if not source.exists(): raise SystemExit(f'Missing required data: {source}')
    if source.is_dir(): shutil.copytree(source,target,dirs_exist_ok=True)
    else: shutil.copy2(source,target)
# Optional: NSE near-month index futures for the Kora Reddy strategies (built from the NSE F&O daily files by
# jay-local/Data/futures/build_bhavcopy_daily.py); the engine runs without it and holds those strategies.
if (root/'futures_daily.csv').exists():
    shutil.copy2(root/'futures_daily.csv',data/'futures_daily.csv')
for item in [data, *data.rglob('*')]: item.chmod(0o755 if item.is_dir() else 0o644)
frontend_data=root/'Context/guru/frontend/data'
if frontend_data.is_dir():
    shutil.copytree(frontend_data,runtime/'frontend-data',dirs_exist_ok=True)
else:
    (runtime/'frontend-data').mkdir(exist_ok=True)
notes=runtime/'notes'
notes.mkdir(exist_ok=True)
# The reader is empty on a fresh installation until the explicit checksum
# stager supplies admitted originals. Never discover/copy the NAS at startup.
(runtime/'books').mkdir(exist_ok=True)
try:
    with (runtime/'book-index.json').open('x') as index:
        index.write('{"version":1,"files":{}}\n')
except FileExistsError:
    pass
for source in [root/'Context/guru/notes', Path.home()/'.local/share/strategy-vault/notes']:
    if source.is_dir():
        for note in source.glob('*.txt'):
            if not (notes/note.name).exists(): shutil.copy2(note,notes/note.name)
# The FUSE share is readable by this CLI but not by Docker's daemon. Stage small
# runtime inputs on local disk; all source data remains in its original location.
shutil.copy2(app/'infra/Caddyfile',runtime/'Caddyfile')
certificate=Path('/usr/local/share/ca-certificates/certificate.crt')
(runtime/'build-ca.crt').write_bytes(certificate.read_bytes() if certificate.exists() else b'')
env=runtime/'.env'
if not env.exists():
    settings={
        'VAULT_DB_PASSWORD':secrets.token_hex(24),
        'BETTER_AUTH_SECRET':secrets.token_hex(32),
        'VAULT_INTERNAL_TOKEN':secrets.token_hex(32),
        'VAULT_ADMIN_EMAIL':'manan@vault.local','VAULT_ADMIN_NAME':'Manan',
        'VAULT_ADMIN_PASSWORD':secrets.token_urlsafe(24),
        'VAULT_PUBLIC_URL':'http://localhost:8180',
        'VAULT_ALLOWED_ORIGINS':'http://127.0.0.1:8180',
        'VAULT_BIND_HOST':'127.0.0.1','VAULT_PORT':'8180',
        'VAULT_CPUSET':','.join(str(cpu) for cpu in sorted(os.sched_getaffinity(0))[:2]),
        'VAULT_RUNTIME_ROOT':str(runtime),
    }
    env.write_text(''.join(f'{key}={value}\n' for key,value in settings.items()))
    env.chmod(0o600)
settings=dict(line.split('=',1) for line in env.read_text().splitlines() if line and not line.startswith('#'))
credentials=runtime/'administrator.txt'
credentials.write_text('Strategy Vault — local administrator\nURL: '+settings['VAULT_PUBLIC_URL']+'\nEmail: '+settings['VAULT_ADMIN_EMAIL']+'\nPassword: '+settings['VAULT_ADMIN_PASSWORD']+'\n')
credentials.chmod(0o600)
(app/'.env').write_text('# Runtime location; secrets are stored on local disk.\nVAULT_RUNTIME_ROOT='+str(runtime)+'\n')
print(f'Runtime inputs ready in {runtime}; existing secrets retained.')
print(f'Administrator credentials: {runtime}/administrator.txt')
