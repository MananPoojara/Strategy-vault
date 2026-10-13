// Live authenticated integration checks. Run only after deploying the Oct 13 accuracy changes.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, open } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const appDirectory = fileURLToPath(new URL('..', import.meta.url));
const envFile = process.env.VAULT_ENV_FILE || `${homedir()}/.cache/strategy-vault-webapp/.env`;
const env = Object.fromEntries((await readFile(envFile, 'utf8')).split(/\r?\n/)
  .filter(line => line && !line.startsWith('#')).map(line => {
    const split = line.indexOf('='); return [line.slice(0, split), line.slice(split + 1)];
  }));
const base = (process.env.VAULT_TEST_URL || env.VAULT_PUBLIC_URL).replace(/\/$/, '');
const runtime = env.VAULT_RUNTIME_ROOT || `${homedir()}/.cache/strategy-vault-webapp`;
const fixture = randomBytes(6).toString('hex');
const key = `accuracy.${fixture}`, password = randomBytes(24).toString('hex');
const emails = ['viewer', 'researcher'].map(role => `${role}.${fixture}@accuracy.vault.local`);
const report = { timestamp: new Date().toISOString(), checks: [], actualResults: {}, fixtureCleanup: false };
const stock = 'basket:nifty50:RELIANCE';
const period = { from: '2020-01-01', to: '2025-12-31', cost: 2, slippage: 3 };
let admin, viewer, researcher, nativeRun, readerId, readerBytes, readerSize;

async function call(path, { cookie = '', method = 'GET', body, origin = base, headers = {}, binary = false } = {}) {
  const response = await fetch(base + path, {
    method, redirect: 'manual', signal: AbortSignal.timeout(45000), body,
    headers: { ...(cookie ? { Cookie: cookie } : {}), ...(!['GET', 'HEAD'].includes(method) ? { Origin: origin } : {}), ...headers },
  });
  if (binary) return { status: response.status, headers: response.headers, bytes: Buffer.from(await response.arrayBuffer()) };
  const text = await response.text();
  return { status: response.status, headers: response.headers, text, json: () => JSON.parse(text) };
}

function endpoint(market, action, values, owner = 'connors') {
  return `/api/${market}/guru/${owner}/${action}?${new URLSearchParams(values)}`;
}

async function signIn(email, secret) {
  const response = await call('/api/auth/sign-in/email', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ email, password: secret }) });
  assert.equal(response.status, 200, 'Sign-in request failed');
  assert.match(response.headers.get('set-cookie'), /httponly/i);
  return response.headers.getSetCookie().map(value => value.split(';')[0]).join('; ');
}

function near(actual, expected, tolerance = 0.000002) {
  assert.ok(Number.isFinite(actual) && Math.abs(actual - expected) <= tolerance, `${actual} differs from independent ${expected}`);
}

function fillReturn(trade, sideBps) {
  return trade.legs.reduce((sum, [, price, weight]) => sum + weight * (trade.dir * (trade.exit_px / price - 1) - 2 * sideBps / 10000), 0);
}

async function blockedRun(id, { ticker = 'NIFTY 50', market = 'in', pattern, extra = {} } = {}) {
  const response = await call(endpoint(market, 'run', { id, ticker, ...period, ...extra }), { cookie: viewer });
  assert.equal(response.status, 400, `${id} must not publish this run`);
  if (pattern) assert.match(response.text, pattern);
  assert.equal(response.text.includes('"equity":['), false);
  return response;
}

await test('Deployed accuracy, instrument routing, arithmetic and local book reading', async t => {
  const check = async (name, body) => t.test(name, async () => {
    try { await body(); report.checks.push({ name, status: 'passed' }); }
    catch (error) { report.checks.push({ name, status: 'failed' }); throw error; }
  });
  try {
    await check('health and anonymous requests preserve authentication', async () => {
      assert.equal((await call('/healthz')).status, 200);
      for (const path of ['/api/app/me', '/api/in/tickers', '/api/app/books/files/not_known']) assert.equal((await call(path)).status, 401);
      admin = await signIn(env.VAULT_ADMIN_EMAIL, env.VAULT_ADMIN_PASSWORD);
    });
    await check('temporary viewer and researcher accounts use existing administrator provisioning', async () => {
      for (const [i, role] of ['viewer', 'researcher'].entries()) {
        const response = await call('/api/app/admin/users', { cookie: admin, method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name: `Accuracy ${role}`, email: emails[i], password, role }) });
        assert.equal(response.status, 201, 'Temporary account creation failed');
      }
      viewer = await signIn(emails[0], password); researcher = await signIn(emails[1], password);
      assert.equal((await call('/api/app/admin/users', { cookie: viewer })).status, 403);
      assert.equal((await call('/api/app/admin/users', { cookie: researcher })).status, 403);
    });
    await check('stock strategy rejects Nifty index and SPY ETF substitutes', async () => {
      await blockedRun('f1a', { pattern: /individual stock|constituent basket/i });
      await blockedRun('f1a', { ticker: 'SPY', market: 'us', pattern: /individual stock|constituent basket/i });
      const readiness = await call(endpoint('in', 'readiness', { id: 'f1a', ticker: 'NIFTY 50', ...period }), { cookie: viewer });
      assert.equal(readiness.status, 200); assert.equal(readiness.json().strategies[0].status, 'needs_data');
    });
    await check('actual Nifty 50 constituent is admitted and retains honest source provenance', async () => {
      const response = await call(endpoint('in', 'run', { id: 'f1a', ticker: stock, ...period }), { cookie: viewer });
      assert.equal(response.status, 200, response.text.slice(0, 300)); nativeRun = response.json();
      assert.equal(nativeRun.provenance.symbol, 'RELIANCE'); assert.equal(nativeRun.provenance.basket, 'nifty50');
      assert.match(nativeRun.provenance.csv_sha256, /^[0-9a-f]{64}$/);
      assert.equal(nativeRun.provenance.quote_accuracy_independently_verified, false);
      assert.match(nativeRun.provenance.basis, /snapshot|survivorship/i);
      assert.ok(nativeRun.trades.length > 10);
      for (const trade of nativeRun.trades) {
        assert.equal(trade.accounting.symbol, 'RELIANCE'); assert.equal(trade.accounting.net_pnl, null); assert.equal(trade.accounting.taxes, null);
      }
    });
    await check('net profit independently sums closed fills; mean/count and compounding remain separate', async () => {
      assert.ok(nativeRun, 'The actual constituent run is required');
      const closed = nativeRun.trades.filter(trade => !trade.open), returns = closed.map(trade => fillReturn(trade, 5));
      const sum = returns.reduce((a, b) => a + b, 0), average = sum / closed.length;
      const compound = nativeRun.trades.reduce((value, trade) => value * (1 + fillReturn(trade, 5)), 1) - 1;
      assert.equal(nativeRun.stats.trades, closed.length);
      near(nativeRun.stats.net_profit, sum); near(nativeRun.stats.closed_return_sum, sum);
      near(nativeRun.stats.avg_net_pnl, average); near(nativeRun.stats.avg_trade, average);
      near(nativeRun.stats.avg_net_pnl * closed.length, sum, (closed.length + 1) * 0.00000051);
      near(nativeRun.stats.compounded_return, compound, 0.00001); near(nativeRun.stats.total_return, compound, 0.00001);
      assert.ok(Math.abs(sum - compound) > 0.00001, 'This real ledger must distinguish arithmetic sum from compounding');
      const grossSum = closed.reduce((value, trade) => value + fillReturn(trade, 0), 0);
      near(nativeRun.gross_stats.net_profit, grossSum);
      near(grossSum - sum, closed.reduce((value, trade) => value + trade.legs.reduce((v, [, , weight]) => v + weight, 0) * 0.001, 0));
      assert.match(nativeRun.accounting_basis, /Open marks are excluded|closed-trade/i);
      report.actualResults.native = { symbol: 'RELIANCE', from: nativeRun.dates[0], to: nativeRun.dates.at(-1), closedPositions: closed.length, closedReturnSum: sum, averageClosedReturn: average, compoundedReturn: compound, sourceStudyReproduced: false };
    });
    await check('index and ETF methods require their actual instrument types', async () => {
      await blockedRun('r3', { ticker: stock, pattern: /actual index|different instrument/i });
      await blockedRun('tps_long', { pattern: /real ETF|not the ETF/i });
      const index = await call(endpoint('in', 'run', { id: 'r3', ticker: 'NIFTY 50', ...period }), { cookie: viewer });
      assert.equal(index.status, 200, index.text.slice(0, 300));
      const etf = await call(endpoint('us', 'run', { id: 'tps_long', ticker: 'SPY', ...period }), { cookie: viewer });
      assert.equal(etf.status, 200, etf.text.slice(0, 300));
      assert.ok(etf.json().dates.length > 100);
    });
    await check('Nifty history starts in 2000 even when an earlier period is requested', async () => {
      const tickers = await call('/api/in/tickers', { cookie: viewer }); assert.equal(tickers.status, 200);
      const nifty = tickers.json().find(item => item.ticker === 'NIFTY 50'); assert.ok(nifty); assert.ok(nifty.first >= '2000-01-01');
      const result = await call(endpoint('in', 'run', { id: 'r3', ticker: 'NIFTY 50', from: '1995-01-01', to: '2001-12-31' }), { cookie: viewer });
      assert.equal(result.status, 200, result.text.slice(0, 300)); assert.ok(result.json().dates[0] >= '2000-01-01');
      report.actualResults.niftyFirst = nifty.first;
    });
    await check('Fourplay, REBO, Turtle Soup and overlapping observations emit no performance', async () => {
      for (const [id, pattern, extra] of [
        ['atm00_fourplay', /sizing|portfolio|allocation/i, {}],
        ['tdperl_rebo', /sequencing|reversal|Intraday/i, {}],
        ['ss_turtle_soup', /sequence|Intraday|recovery/i, {}],
        ['ss_study_close_upper_90', /Overlapping|overlap/i, { 'p.overlap': 1 }],
      ]) {
        await blockedRun(id, { pattern, extra });
        const readiness = await call(endpoint('in', 'readiness', { id, ticker: 'NIFTY 50', ...period, ...extra }), { cookie: viewer });
        assert.equal(readiness.status, 200); assert.notEqual(readiness.json().strategies[0].status, 'ready');
        const optimization = await call(endpoint('in', 'optimize', { id, ticker: 'NIFTY 50', ...period, ...extra }), { cookie: researcher });
        assert.equal(optimization.status, 400); assert.match(optimization.text, pattern);
      }
      await blockedRun('rgc_volbo', { pattern: /source review|futures|instrument/i });
      await blockedRun('r3r4_lowprice', { ticker: stock, pattern: /USD|currency|INR/i });
    });
    await check('catalogue distinguishes instrument admission and approximate table comparison from certification', async () => {
      const result = await call('/api/in/guru/connors/strategies', { cookie: viewer }); assert.equal(result.status, 200);
      const entries = result.json(), foundation = entries.find(item => item.id === 'f1a');
      assert.equal(foundation.instrument_policy.requirement, 'individual_stock');
      assert.equal(foundation.instrument_policy.source_performance_reproduced, false);
      assert.ok(entries.every(item => item.gate?.verdict !== 'verified'));
      const compared = entries.find(item => item.gate?.verdict === 'table_compared');
      assert.ok(compared); assert.ok(compared.gate.book); assert.ok(compared.gate.ours);
    });
    await check('two real native optimization candidates preserve captured costs and arithmetic totals', async () => {
      const response = await call(endpoint('in', 'optimize', { id: 'f1a', ticker: stock, ...period, 'g.hold': '4:5:1' }), { cookie: researcher });
      assert.equal(response.status, 200, response.text.slice(0, 300));
      const result = response.json(); assert.equal(result.combos, 2); assert.equal(result.rows.length, 2); assert.ok(result.threads <= 2);
      assert.equal(result.costs.fees_bps, 2); assert.equal(result.costs.slippage_bps, 3); assert.equal(result.provenance.symbol, 'RELIANCE');
      const holdColumn = result.cols.indexOf('p.hold'), sumColumn = result.cols.indexOf('net_profit'), avgColumn = result.cols.indexOf('avg_net_pnl'), countColumn = result.cols.indexOf('trades');
      assert.ok([holdColumn, sumColumn, avgColumn, countColumn].every(index => index >= 0));
      for (const row of result.rows) {
        const detail = await call(endpoint('in', 'run', { id: 'f1a', ticker: stock, ...period, 'p.hold': row[holdColumn] }), { cookie: viewer });
        assert.equal(detail.status, 200); const actual = detail.json();
        const closed = actual.trades.filter(trade => !trade.open);
        near(row[sumColumn], closed.reduce((value, trade) => value + fillReturn(trade, 5), 0));
        assert.equal(row[countColumn], closed.length); near(row[avgColumn], actual.stats.avg_net_pnl);
      }
      report.actualResults.optimization = { combinations: result.combos, threads: result.threads, symbol: 'RELIANCE', feesBpsPerSide: 2, slippageBpsPerSide: 3 };
    });
    await check('viewer optimization permissions cover both markets and all three owners', async () => {
      for (const market of ['in', 'us']) for (const owner of ['connors', 'ross', 'chande']) {
        const result = await call(endpoint(market, 'optimize', { id: 'f1a', ticker: stock }, owner), { cookie: viewer });
        assert.equal(result.status, 403, `${market}/${owner}`);
      }
      const crossSite = await call(endpoint('in', 'optimize', { id: 'f1a', ticker: stock }), { cookie: researcher, headers: { 'Sec-Fetch-Site': 'cross-site' } });
      assert.equal(crossSite.status, 403);
    });
    await check('notes, per-user settings and write-origin checks remain intact', async () => {
      const saved = await call(`/api/in/notes?key=${key}`, { cookie: researcher, method: 'POST', body: 'Accuracy verification fixture.' });
      assert.equal(saved.status, 200); assert.equal((await call(`/api/us/notes?key=${key}`, { cookie: viewer })).text, 'Accuracy verification fixture.');
      assert.equal((await call(`/api/in/notes?key=${key}`, { cookie: viewer, method: 'POST', body: 'Overwrite' })).status, 403);
      for (const origin of ['', 'https://untrusted.example']) assert.equal((await call(`/api/in/notes?key=${key}`, { cookie: researcher, method: 'POST', origin, body: 'Overwrite' })).status, 403);
      const theme = await call('/api/app/settings', { cookie: researcher, method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: '{"theme":"midnight"}' });
      assert.equal(theme.status, 200); assert.equal((await call('/api/app/me', { cookie: researcher })).json().settings.theme, 'midnight');
      assert.equal((await call('/api/app/me', { cookie: viewer })).json().settings.theme, 'aqua');
      assert.equal((await call('/api/app/settings', { cookie: researcher, method: 'PUT', origin: 'https://untrusted.example', headers: { 'Content-Type': 'application/json' }, body: '{"theme":"forest"}' })).status, 403);
    });
    await check('actual staged PDF metadata and range bytes match the read-only local file', async () => {
      const manifest = JSON.parse(await readFile(join(runtime, 'book-index.json'), 'utf8'));
      for (const [id, item] of Object.entries(manifest.files).slice(0, 100)) {
        let handle;
        try {
          handle = await open(join(runtime, 'books', item.path), 'r');
          const bytes = Buffer.alloc(1024), result = await handle.read(bytes, 0, bytes.length, 0);
          if (bytes.subarray(0, 5).toString('ascii') !== '%PDF-') continue;
          readerId = id; readerBytes = bytes.subarray(0, result.bytesRead); readerSize = (await handle.stat()).size; break;
        } finally { await handle?.close(); }
      }
      assert.ok(readerId, 'A staged manifest PDF is required');
      const path = `/api/app/books/files/${readerId}`;
      const metadata = await call(path + '/info', { cookie: viewer }); assert.equal(metadata.status, 200);
      assert.equal(metadata.json().sourceIdentityVerified, false); assert.equal(metadata.json().bytes, readerSize);
      const bytes = await call(path, { cookie: viewer, binary: true, headers: { Range: `bytes=0-${readerBytes.length - 1}` } });
      assert.equal(bytes.status, 206); assert.deepEqual(bytes.bytes, readerBytes);
      assert.equal(bytes.headers.get('content-type'), 'application/pdf'); assert.match(bytes.headers.get('cache-control'), /private.*no-store/);
      assert.equal(bytes.headers.get('content-range'), `bytes 0-${readerBytes.length - 1}/${readerSize}`);
      assert.equal(bytes.headers.get('content-length'), String(readerBytes.length)); assert.equal(bytes.headers.get('accept-ranges'), 'bytes');
      assert.match(bytes.headers.get('content-disposition'), /^inline;/);
      // A negotiated gzip representation legitimately has no known length.
      // Request the identity representation to compare the original file size.
      const head = await call(path, { cookie: viewer, method: 'HEAD', headers: { 'Accept-Encoding': 'identity' } });
      assert.equal(head.status, 200); assert.equal(head.text, ''); assert.equal(head.headers.get('content-length'), String(readerSize));
      const invalid = await call(path, { cookie: viewer, headers: { Range: `bytes=${readerSize}-` } });
      assert.equal(invalid.status, 416); assert.equal(invalid.headers.get('content-range'), `bytes */${readerSize}`);
      for (const id of ['not_known', '%2e%2e%2foutside', 'a%5cb', 'a%0ab']) assert.equal((await call(`/api/app/books/files/${id}`, { cookie: viewer })).status, 404);
      assert.equal((await call(path)).status, 401);
      report.actualResults.localReader = { id: readerId, bytesCompared: readerBytes.length, fileSize: readerSize, sourceIdentityVerified: false };
    });
    await check('fixture session becomes invalid immediately on sign-out', async () => {
      assert.equal((await call('/api/auth/sign-out', { cookie: viewer, method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' })).status, 200);
      assert.equal((await call('/api/app/me', { cookie: viewer })).status, 401);
      assert.equal((await call(`/api/app/books/files/${readerId || 'not_known'}`, { cookie: viewer })).status, 401);
    });
  } finally {
    try {
      for (const cookie of [viewer, researcher, admin]) {
        if (cookie) await call('/api/auth/sign-out', { cookie, method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' }).catch(() => {});
      }
      // All identifiers are this run's random hex suffix; no real user or saved data is selected.
      const sql = `DELETE FROM vault_note_history WHERE note_key='${key}'; DELETE FROM vault_notes WHERE note_key='${key}'; DELETE FROM vault_audit WHERE entity='${key}' OR entity IN (SELECT id FROM "user" WHERE email IN ('${emails.join("','")}')); DELETE FROM "user" WHERE email IN ('${emails.join("','")}');`;
      execFileSync('docker', ['compose', '--project-directory', appDirectory, '--env-file', envFile, 'exec', '-T', 'postgres', 'psql', '-U', 'vault', '-d', 'vault', '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: ['ignore', 'pipe', 'pipe'] });
      report.fixtureCleanup = true;
    } finally {
      report.finished = new Date().toISOString();
      await writeFile(new URL('../verification/accuracy-2026-10-13-api-report.json', import.meta.url), JSON.stringify(report, null, 2) + '\n');
    }
  }
});
