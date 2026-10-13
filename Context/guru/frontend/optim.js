// Optimization research on the strategy page: parameter ranges, Rust grid run, Claude's conclusion (top 3 by the
// percentile / group / cross-comparison method, computed in Rust), YT's conclusion and an Excel workbook whose stats
// are recomputed with Excel formulas. Uses the page's globals (API, S, R, PUB, TICKERS, esc, pct, fx, …).
const OPT = { schema: null, schemaSync: null, grids: {}, cache: {}, cc: {}, last: null };
const OPT_LIMIT = 50000, CC_LIMIT = 10000; // manual search limit; Claude's conclusion searches up to 10,000 settings per window
const optSchema = async () => OPT.schemaSync = await (OPT.schema || (OPT.schema = Promise.all(guruIds().map(g=>fetch(`${API_IN}/guru/${g}/params`).then(r=>{if(!r.ok)throw new Error('Parameter schema unavailable');return r.json();}))).then(sets=>Object.assign({},...sets)).catch(()=>({}))));
const decodeP = s => Object.fromEntries((s || '').split(',').filter(Boolean).map(kv => kv.split(':')).map(([k, v]) => [k, +v]));
const encodeP = p => Object.entries(p).map(([k, v]) => `${k}:${v}`).join(',');
const optKey = (id, tk) => [id, tk, S.from, S.to, S.cost, S.slippage, S.rollover, S.region, S.basket].join('|');
const nVals = g => g.step > 0 && g.max >= g.min ? Math.floor((g.max - g.min) / g.step + 1e-9) + 1 : 0;
const pLabel = (id, k) => (OPT.schemaSync?.[id] || []).find(x => x.key === k)?.label || k;
// compact column names for tables
const SHORT = { trend: 'Trend SMA', rsi_n: 'RSI period', exit_rsi: 'Exit RSI', rsi_lt: 'Entry RSI <', rsi_gt: 'Entry RSI >', hold: 'Hold bars', ma: 'Short SMA', low_n: 'Lookback', limit: 'Limit %',
  fall: 'Falling days', first_lt: 'First RSI <', add_fall: 'Add after', down: 'Down days', add_pct: 'Add %', stretch: 'Stretch %', limit_min: 'Min limit %', profit: 'Profit %', hh_n: 'High lookback',
  atr_k: 'ATR ×', band: 'Band %', stop: 'Stop %', hv: 'HV >', adx: 'ADX >', lf: 'Liquidity >', days: 'Days', up: 'Up days', rise: 'Rise %', dom: 'From day', bars: 'Bars', slow_n: '2nd RSI period',
  slow_lt: '2nd RSI <', look: 'Lookback', sum_n: 'Summed RSI', sum_lt: 'Sum <', loc: 'Close in low %', pctb_n: '%b period', pctb_lt: '%b <', di_n: '+DI period', di_lt: '+DI <', vix_rsi: 'VIX RSI >' };
const sLabel = k => SHORT[k] || k;
const n0 = x => (x ?? 0).toLocaleString('en-IN');
const bookDefaults = id => Object.fromEntries((OPT.schemaSync?.[id] || []).map(x => [x.key, x.def]));
const sameP = (a, b) => Object.keys(b).every(k => Math.abs((a[k] ?? b[k]) - b[k]) < 1e-9);

// ---------------------------------------------------------------- book periods (All / Before book / After book)
const tkSpan = tk => TICKERS.find(t => t.ticker === tk) || TICKERS[0];
const BOOKWIN = () => ({ all: { from: '', to: '' }, before: { from: '', to: `${PUB - 1}-12-31` }, after: { from: `${PUB}-01-01`, to: '' } });
const periodKind = (from = S.from, to = S.to) => !from && !to ? 'all' : !from && to === `${PUB - 1}-12-31` ? 'before' : from === `${PUB}-01-01` && !to ? 'after' : 'custom';
const PERIOD_NAME = { all: 'All', before: 'Before book', after: 'After book', custom: 'Custom dates' };

// ---------------------------------------------------------------- the stats the conclusion shows (user's list) and ranks on (five)
const CC_STATS = [
  ['cagr', 'CAGR', v => pct(v, 2, true), 1], ['net_profit', 'Net profit', v => pct(v, 2, true), 1], ['max_dd', 'Max DD', v => pct(v, 2), 0], ['win_rate', 'Win %', v => pct(v, 2), 0],
  ['sharpe', 'Sharpe', v => fx(v), 1], ['monthly_thickness', 'Monthly thickness', v => pct(v, 2, true), 1], ['trades', 'Trades', v => fx(v, 0), 0],
  ['expectancy_ratio', 'Expectancy ratio', v => fx(v), 1], ['car_mdd', 'CAR/MDD', v => fx(v), 1]];
const RANK_KEYS = ['cagr', 'sharpe', 'monthly_thickness', 'expectancy_ratio', 'car_mdd'];
const RANK_LABEL = { cagr: 'CAGR', sharpe: 'Sharpe', monthly_thickness: 'Monthly thickness', expectancy_ratio: 'Expectancy ratio', car_mdd: 'CAR/MDD' };
/** A result row of /optimize as {p,m}; preserve the engine's closed-return sum and alias only CAR/MDD. */
function rowOf(res, r) {
  const nd = res.dims.length, p = { ...bookDefaults(res.id), ...(res.fixed || {}) }, m = {};
  res.dims.forEach((d, j) => p[d.key] = r[j]);
  res.cols.forEach((c, i) => { if (i >= nd) m[c] = r[i]; });
  m.car_mdd = m.mar; return { p, m };
}
const bookRow = res => { const m = {}; res.cols.slice(res.dims.length).forEach((c, i) => m[c] = res.book[i]); m.car_mdd = m.mar; return { p: bookDefaults(res.id), m }; };
const statsM = s => ({ ...s, car_mdd: s.mar });
/** Row i of an /optimize result: full results carry `rows`, slim ones only the ranked candidates in `picked`. */
const rowAt = (res, i) => res.rows.length ? res.rows[i] : res.picked[i];

// Banner when the page runs custom (non-book) settings.
function optimBanner(v) {
  const p = decodeP(S.p); if (!Object.keys(p).length) return '';
  const book = bookDefaults(v.id);
  return `<div class="pbanner"><span class="olabel">Custom settings</span><span>${Object.entries(p).map(([k, x]) => `<span class="oset">${esc(sLabel(k))} <b>${x}</b>${book[k] !== x ? ` <span class="muted">book ${book[k] ?? '–'}</span>` : ''}</span>`).join(' ')}</span><a href="${href(R.path, { ...S, p: '' })}">Back to book settings</a></div>`;
}

// ---------------------------------------------------------------- loader: a book flipping its pages
const bookLoader = (t = 'Loading…') => `<div class="bload" role="status" aria-live="polite"><div class="bflip" aria-hidden="true"><i class="bcv l"></i><i class="bcv r"></i><i class="bpg"></i><i class="bpg"></i><i class="bpg"></i></div><p>${esc(t)}</p></div>`;
const minWait = (t0, ms) => new Promise(ok => setTimeout(ok, Math.max(0, ms - (performance.now() - t0))));

async function optimSection(v, tk, d, detailed = true) {
  OPT.schemaSync = await optSchema();
  const ps = OPT.schemaSync[v.id] || [];
  const opt = detailed && ps.length ? (() => { const g = OPT.grids[v.id] || (OPT.grids[v.id] = OptimCore.defaultGrid(ps, OPT_LIMIT));
    return `<section class="box" id="optim"><div class="box-h">Optimization <small>Tick the settings to vary. Unticked settings stay at the book value. Runs on the period selected above.</small></div>
    <table class="ogrid"><thead><tr><th></th><th>Setting</th><th class="n">Book</th><th class="n">From</th><th class="n">To</th><th class="n">Step</th><th class="n">Values</th></tr></thead><tbody>
    ${ps.map(p => { const x = g[p.key]; return `<tr data-k="${p.key}" class="${x.on ? '' : 'offrow'}"><td><input type="checkbox" data-f="on" ${x.on ? 'checked' : ''} aria-label="Vary ${esc(p.label)}"></td><td>${esc(p.label)}</td><td class="n">${p.def}</td>
      ${['min', 'max', 'step'].map(f => `<td class="n"><input type="number" data-f="${f}" value="${x[f]}" step="any"></td>`).join('')}<td class="n muted" data-cnt>${nVals(x)}</td></tr>`; }).join('')}</tbody></table>
    <form class="obar" id="oform"><span class="grow"></span><span class="ocount" id="ocount"></span><button class="dbtn primary sm" type="submit" id="orun">Run optimization</button></form>
    <p class="ostat" id="otime"></p><div id="opt-out"></div></section>`; })() : '';
  const claude = detailed ? `<section class="box" id="claude-c"><div class="box-h">Claude’s conclusion <small>Top 3 chosen once on all the data (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD), shown for ${esc(PERIOD_NAME[periodKind()])} and split before / after the book</small></div><p class="rnote">This summary is calculated locally from Rust backtests. Opening a strategy does not call an AI model or use AI tokens. Parameter selection is research, not proof of future performance or reproduction of the book.</p><div id="cc-body">${ps.length ? bookLoader('Rust is testing the parameter grid…') : '<p class="muted">This rule has no settings to vary, so there is nothing to rank.</p>'}</div></section>` : `<section class="box" id="claude-c"><div class="box-h">Claude’s conclusion <small>Parameter research is available on the optimization page</small></div><p>Open Optimization to run the existing full-history parameter ranking, read its comparisons and download its workbook.</p><a class="dbtn sm" href="${href('/o/'+encodeURIComponent(v.id),{...S})}">Open parameter research</a></section>`;
  return `${opt}${claude}${gptSection(v,tk,d)}
    <section class="box" id="yt-c"><div class="box-h">YT’s conclusion <small>Our own call on the settings, saved in the vault</small></div>
      <textarea id="yt-text" class="yt" rows="6" placeholder="Which settings we trade, on which index, why they differ from the book, and what would make us stop."></textarea>
      <div class="ytbar"><button class="dbtn sm" id="yt-save">Save</button><span class="muted" id="yt-msg"></span></div></section>`;
}

function optCount() {
  const el = $('#optim'); if (!el) return 0;
  let n = 1, on = 0; el.querySelectorAll('tr[data-k]').forEach(tr => { const x = OPT.grids[R.id][tr.dataset.k], k = nVals(x); tr.querySelector('[data-cnt]').textContent = k || '–'; tr.classList.toggle('offrow', !x.on); if (x.on) { n *= k; on++; } });
  $('#ocount').innerHTML = on ? `<b class="${n > OPT_LIMIT || !n ? 'neg' : ''}">${n0(n)}</b> combinations${n > OPT_LIMIT ? `, limit ${n0(OPT_LIMIT)}` : ''}` : 'Tick a setting to vary';
  $('#orun').disabled = !on || !n || n > OPT_LIMIT || (window.VAULT_APP && window.VAULT_USER?.role === 'viewer'); return n;
}

function optimInit() {
  if (!STRAT) return;
  const { v, tk } = STRAT, el = $('#optim');
  if (el) {
    const g = OPT.grids[v.id];
    el.addEventListener('input', e => { const tr = e.target.closest('tr[data-k]'); if (!tr) return; const f = e.target.dataset.f; g[tr.dataset.k][f] = f === 'on' ? e.target.checked : +e.target.value; optCount(); });
    $('#oform').addEventListener('submit', e => { e.preventDefault(); runOpt(); });
    optCount();
    const c = OPT.cache[optKey(v.id, tk)]; if (c) showOpt(c.res);
  }
  if ($('#cc-body') && (OPT.schemaSync[v.id] || []).length && (!window.VAULT_APP || window.VAULT_USER?.role !== 'viewer')) conclusionRun(v, tk);
  const key = `${v.id}.${S.region}`, noteText = $('#yt-text'), noteMessage = $('#yt-msg');
  fetch(`${API_IN}/notes?key=${key}`).then(r => r.ok ? r.text() : '').then(t => { if (noteText && $('#yt-text') === noteText) noteText.value = t; }).catch(() => {});
  if (window.VAULT_APP && window.VAULT_USER?.role === 'viewer') { if ($('#gpt-body')) $('#gpt-body').innerHTML = '<p class="muted">Parameter research requires researcher access.</p>'; if ($('#cc-body')) $('#cc-body').innerHTML = '<p class="muted">Automatic parameter search requires researcher access. You can read saved conclusions below and run the book settings above.</p>'; if ($('#yt-save')) { $('#yt-save').disabled = true; $('#yt-save').title = 'Researcher access is required to save conclusions'; } if ($('#yt-text')) $('#yt-text').readOnly = true; if ($('#orun')) { $('#orun').disabled = true; $('#orun').title = 'Researcher access is required to optimize'; } return; }
  if ($('#yt-save')) $('#yt-save').onclick = async () => { const m = noteMessage; m.textContent = 'Saving…';
    try { const r = await fetch(`${API_IN}/notes?key=${key}`, { method: 'POST', headers: { 'Content-Type': 'text/plain;charset=utf-8' }, body: noteText.value }); m.textContent = r.ok ? `Saved at ${new Date().toLocaleTimeString()}` : 'Save failed: ' + await r.text(); } catch (err) { m.textContent = 'Save failed: ' + err.message; } };
}

const optQuery = (v, tk, w, g, slim) => { const q = new URLSearchParams({ id: v.id, ticker: tk, from: w.from || '', to: w.to || '', cost: S.cost || '0',slippage:S.slippage||'0',rollover:S.rollover||'0', ...(slim ? { slim: '1' } : {}) });
  Object.entries(g).forEach(([k, x]) => { if (x.on) q.set('g.' + k, `${x.min}:${x.max}:${x.step}`); }); return q; };
async function fetchOpt(q, api=API) { const r = await fetch(enginePath(q.get('id'),'optimize',api)+'?'+q), b = await r.text(); if (!r.ok) { let message = b; try { message = JSON.parse(b).error || b; } catch {} throw new Error(message); } return JSON.parse(b); }

async function runOpt() {
  const { v, tk } = STRAT, g = OPT.grids[v.id], key=optKey(v.id,tk), out=$('#opt-out');
  const n = optCount(); $('#orun').disabled = true; $('#otime').textContent = `Running ${n0(n)} backtests…`;
  $('#opt-out').innerHTML = bookLoader(`Running ${n0(n)} backtests in Rust…`);
  const t0 = performance.now();
  try {
    const res = await fetchOpt(optQuery(v, tk, { from: S.from, to: S.to }, g, false)); await minWait(t0, 900); res.rt = performance.now() - t0;
    OPT.cache[key] = { res }; if (STRAT && optKey(STRAT.v.id,STRAT.tk)===key && $('#opt-out')===out) showOpt(res);
  } catch (e) { if ($('#opt-out')===out) {out.innerHTML = ''; $('#otime').innerHTML = `<span class="neg">${esc(e.message)}</span>`;} }
  finally { if ($('#orun')) optCount(); }
}

const applyLink = p => `<button class="olink" data-apply="${esc(encodeP(p))}">Apply</button>`;
const setLine = (p, def) => Object.keys(def).map(k => `<span class="oset">${esc(sLabel(k))} <b class="${Math.abs((p[k] ?? def[k]) - def[k]) > 1e-9 ? 'chg' : ''}">${p[k] ?? def[k]}</b></span>`).join(' ');
const statCells = m => CC_STATS.map(([k, , f, signed]) => `<td class="n ${k === 'max_dd' ? 'neg' : signed ? cls(m[k]) : ''}">${f(m[k])}</td>`).join('');
const statHead = () => CC_STATS.map(([, t]) => `<th class="n">${t}</th>`).join('');

// Manual optimization result: the book and the top 3 by Claude's ranking, the parameter map, the top 25 by score.
function showOpt(res) {
  const { v } = STRAT, out = $('#opt-out'); if (!out) return;
  const def = bookDefaults(v.id), rk = res.rank, rows = res.rows.map(r => rowOf(res, r)), top = (rk.top3 || []).map(c => ({ ...rows[c.row], c }));
  $('#otime').textContent = `${n0(res.combos)} settings on ${res.from} to ${res.to} in ${res.ms} ms (Rust, ${res.threads} threads). ${n0(rk.eligible)} had ${rk.min_trades}+ trades (${fx(rk.per_year, 1)} a year) and were ranked.`;
  const tbl = `<table class="tbl ocmp cc-tbl"><thead><tr><th></th><th>Settings</th>${statHead()}<th class="n">Score</th><th></th></tr></thead><tbody>
    <tr class="book"><td>Book</td><td class="cset">${setLine({}, def)}</td>${statCells(bookRow(res).m)}<td class="n">–</td><td></td></tr>
    ${top.map((t, k) => `<tr class="${k ? '' : 'pick'}"><td>#${k + 1}</td><td class="cset">${setLine(t.p, def)}</td>${statCells(t.m)}<td class="n">${fx(t.c.score, 1)}</td><td class="n">${sameP(t.p, def) ? '<span class="muted">book</span>' : applyLink(t.p)}</td></tr>`).join('')}</tbody></table>`;
  const dims = res.dims, big = [...dims].sort((a, b) => b.values.length - a.values.length);
  OPT.hx ??= {}; const hx = OPT.hx[v.id] && dims.find(d => d.key === OPT.hx[v.id][0]) ? OPT.hx[v.id] : [big[0]?.key, big[1]?.key || '', 'sharpe'];
  const sel = (id, cur, opts) => `<select id="${id}">${opts.map(([k, t]) => `<option value="${k}" ${k === cur ? 'selected' : ''}>${esc(t)}</option>`).join('')}</select>`;
  const dOpts = dims.map(d => [d.key, sLabel(d.key)]);
  const map = dims.length ? `<div class="ohead"><h4>Parameter map</h4><span class="osel">${sel('hx-x', hx[0], dOpts)} across, ${sel('hx-y', hx[1], [['', 'nothing'], ...dOpts])} down, showing ${sel('hx-m', hx[2], [['sharpe', 'Sharpe'], ['cagr', 'CAGR'], ['max_dd', 'Max DD'], ['mar', 'CAR/MDD'], ['expectancy_ratio', 'Expectancy ratio'], ['monthly_thickness', 'Monthly thickness'], ['trades', 'Trades'], ['win_rate', 'Win %']])}</span></div><div id="hx"></div>` : '';
  out.innerHTML = `<div class="ohead"><h4>Book and Claude’s top 3 on this run</h4><span><button class="dbtn primary sm" id="grid-xlsx">Download all ${n0(res.combos)} rows</button> <button class="dbtn sm" id="oxlsx">Conclusion workbook</button> <button class="dbtn sm" id="gpt-grid-review">GPT analysis of this grid</button></span></div>${tbl}
    <p class="cap">Score = average percentile on the five ranking stats (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD) among the ${n0(rk.eligible)} settings with ${rk.min_trades}+ trades (${fx(rk.per_year, 1)} a year). Highlighted values differ from the book.</p>${map}`;
  const an = { res, rows, pick: top[0], def };
  const drawHx = () => { OPT.hx[v.id] = [$('#hx-x').value, $('#hx-y').value, $('#hx-m').value]; $('#hx').innerHTML = heatGrid(an, ...OPT.hx[v.id]); };
  if (dims.length) { drawHx(); ['#hx-x', '#hx-y', '#hx-m'].forEach(s => $(s).onchange = drawHx); }
  out.onclick = e => { const b_ = e.target.closest('[data-apply]'); if (b_) go(R.path, { ...S, p: b_.dataset.apply }); };
  if ($('#grid-xlsx')) $('#grid-xlsx').onclick=async()=>{const b=$('#grid-xlsx');b.disabled=true;try{await exportFullGrid(res,v,STRAT.tk);}catch(err){b.textContent='Export failed: '+err.message;}finally{b.disabled=false;}};
  if ($('#oxlsx')) $('#oxlsx').onclick = async () => { const b = $('#oxlsx'); b.disabled = true; b.textContent = 'Building Excel…';
    const tk=STRAT.tk,win={from:S.from,to:S.to},context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first,selectionLabel:'Manual grid window'};
    try { await exportXlsx(await conclusionData(v, tk, res, win, context)); b.textContent = 'Download Excel'; } catch (err) { b.textContent = 'Excel failed: ' + err.message; } finally { b.disabled = false; } };
  if ($('#gpt-grid-review')) $('#gpt-grid-review').onclick=async()=>{
    const b=$('#gpt-grid-review'),body=$('#gpt-body'),tk=STRAT.tk,key=optKey(v.id,tk),win={from:S.from,to:S.to},context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first,selectionLabel:'Manual grid window'};
    OPT.gptGeneration=(OPT.gptGeneration||0)+1;b.disabled=true;b.textContent='Comparing the recorded grid…';
    try{const D=await conclusionData(v,tk,res,win,context);if($('#gpt-body')===body&&optKey(v.id,tk)===key){OPT.last=D;gptConclusionWire(D);body.scrollIntoView({block:'start',behavior:'smooth'});}b.textContent='GPT analysis of this grid';}catch(e){b.textContent='Analysis failed: '+e.message;}finally{b.disabled=false;}
  };
  anim(out, { opacity: [0, 1], transform: ['translateY(6px)', 'translateY(0)'] }, { duration: .3 });
}

// 2-D slice of the grid: other parameters held at the #1 setting (or the book).
function heatGrid(an, xk, yk, mk) {
  const res = an.res, dims = res.dims, at = an.pick ? an.pick.p : an.def, xd = dims.find(d => d.key === xk), yd = dims.find(d => d.key === yk && d.key !== xk);
  const look = new Map(an.rows.map(r => [dims.map(d => r.p[d.key]).join('|'), r]));
  const get = (xv, yv) => look.get(dims.map(d => d.key === xk ? xv : yd && d.key === yd.key ? yv : at[d.key]).join('|'));
  const ys = yd ? yd.values : [null], vals = ys.flatMap(y => xd.values.map(x => get(x, y)?.m[mk])).filter(isFinite), cap = Math.max(1e-9, ...vals.map(Math.abs));
  const f = mk === 'trades' ? x => fx(x, 0) : /cagr|max_dd|win_rate|thickness/.test(mk) ? x => (x * 100).toFixed(1) : x => fx(x);
  const bg = x => x == null || !isFinite(x) ? 'transparent' : `color-mix(in srgb, ${mk !== 'max_dd' && x >= 0 ? 'var(--chart)' : 'var(--red)'} ${Math.round(Math.min(1, Math.abs(x) / cap) * 42 + 4)}%, transparent)`;
  const fixed = dims.filter(d => d.key !== xk && d.key !== yd?.key).map(d => `${esc(sLabel(d.key))} ${at[d.key]}`).join(', ');
  return `<div class="hxwrap"><table class="oheat"><thead><tr><th class="y">${yd ? esc(sLabel(yd.key)) : ''}</th>${xd.values.map(x => `<th>${x}</th>`).join('')}</tr></thead><tbody>
    ${ys.map(y => `<tr><th class="y">${y ?? ''}</th>${xd.values.map(x => { const r = get(x, y), val = r?.m[mk]; return `<td style="background:${bg(val)}" class="${r && sameP(r.p, an.def) ? 'hbook' : ''} ${r && an.pick && r.p === an.pick.p ? 'hpick' : ''}" title="${r ? esc(dims.map(d => `${sLabel(d.key)} ${r.p[d.key]}`).join(', ')) + `, ${r.m.trades} trades` : ''}"${r ? ` data-apply="${esc(encodeP(r.p))}"` : ''}>${val == null ? '–' : f(val)}</td>`; }).join('')}</tr>`).join('')}</tbody></table></div>
    <p class="cap"><span class="key kpick"></span>#1 <span class="key kbook"></span>book setting. Click a cell to apply it.${fixed ? ` Other settings held at ${fixed}.` : ''}</p>`;
}

// ---------------------------------------------------------------- Claude's conclusion
// One search on ALL the data picks the top 3. The same settings (and the book's) are then shown on the period selected
// above, and split before / after the book's publication: the settings never change with the period.
async function conclusionRun(v, tk) {
  const el = $('#cc-body'); if (!el) return;
  const key = [API, v.id, tk, S.cost, S.slippage, S.rollover].join('|'), viewKey=optKey(v.id,tk), context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first}, win={from:S.from,to:S.to}, t0 = performance.now(), gptGeneration=OPT.gptGeneration||0;
  el.innerHTML = bookLoader('Claude is testing every setting on all the data…');
  try {
    const res = await (OPT.cc[key] || (OPT.cc[key] = fetchOpt(optQuery(v, tk, { from: '', to: '' }, OptimCore.defaultGrid(OPT.schemaSync[v.id], CC_LIMIT), true),context.api)));
    const data = await conclusionData(v, tk, res, win, context);
    await minWait(t0, 1200);
    if (STRAT?.v.id !== v.id || STRAT.tk !== tk || optKey(v.id,tk)!==viewKey || $('#cc-body')!==el) return;
    $('#cc-body').innerHTML = conclusionHtml(data); conclusionWire(data); if(gptGeneration===(OPT.gptGeneration||0)){OPT.last = data;gptConclusionWire(data);}
    anim($('#cc-body'), { opacity: [0, 1], transform: ['translateY(6px)', 'translateY(0)'] }, { duration: .35 });
  } catch (e) { delete OPT.cc[key]; if (STRAT?.v.id === v.id && STRAT.tk === tk && optKey(v.id,tk)===viewKey && $('#cc-body')===el) {if($('#gpt-body'))$('#gpt-body').innerHTML=`<p class="muted">Parameter analysis unavailable: ${esc(e.message)}</p>`;$('#cc-body').innerHTML = `<div class="empty"><b>No conclusion for this rule on ${esc(LABEL[tk] || tk)}</b>${esc(e.message)}</div>`;} }
}

/** Book + top 3 of `res`: full runs on the viewed window `win`, and the same settings before / after the book. */
async function conclusionData(v, tk, res, win, context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first}) {
  const {api,cost,slippage='0',rollover='0',pub,us,first}=context;
  const def = bookDefaults(v.id), W = {all:{from:'',to:''},before:{from:'',to:`${pub-1}-12-31`},after:{from:`${pub}-01-01`,to:''}};
  const top = (res.rank.top3 || []).map((c, k) => ({ ...rowOf(res, rowAt(res, c.row)), c, name: `#${k + 1}` }));
  const runs = [{ name: 'Book', p: def, book: true }, ...top];
  const stat = async (r, w) => { const d = await runData(v.id, tk, w.from, w.to, cost, r.book ? '' : encodeP(r.p),api,{slippage,rollover}); return d?.error ? { error: d.error } : { d, m: statsM(d.stats) }; };
  await Promise.all(runs.map(async r => {
    const cur = await stat(r, win); if (cur.error) throw new Error(cur.error); r.d = cur.d; r.m = cur.m;
    [r.before, r.after] = await Promise.all([stat(r, W.before), stat(r, W.after)]);
  }));
  const vixFirst = us ? '1990' : '2009', start = v.needs_vix && vixFirst > first.slice(0, 4) ? vixFirst : first.slice(0, 4);
  const noBefore = +start > pub - 1 ? `${v.needs_vix && start === vixFirst ? (us ? 'VIX' : 'India VIX') : LABEL[tk] || tk} data starts in ${start}, after the book (${pub})` : null;
  return { v, tk, res, runs, top, def, win, noBefore, cost:Number(cost || 0),slippage:Number(slippage),rollover:Number(rollover), region:us?'us':'in', context:{...context}, selectionLabel:context.selectionLabel || 'All history', kind: !win.from&&!win.to?'all':!win.from&&win.to===`${pub-1}-12-31`?'before':win.from===`${pub}-01-01`&&!win.to?'after':'custom', pub, made: new Date().toISOString().slice(0, 10) };
}

const dSet = (p, def) => { const ch = Object.keys(def).filter(k => Math.abs((p[k] ?? def[k]) - def[k]) > 1e-9); return ch.length ? ch.map(k => `${sLabel(k)} ${p[k]} (book ${def[k]})`).join(', ') : 'the book settings'; };
const holdText = (a, b) => !Number.isFinite(a) || !Number.isFinite(b) ? '' : b > a ? 'Sharpe increased after the book' : b < a ? 'Sharpe decreased after the book' : 'Sharpe unchanged after the book';
const baText = (r, pub) => r.before?.m && r.after?.m ? `Sharpe ${fx(r.before.m.sharpe)} before ${pub}, ${fx(r.after.m.sharpe)} after (CAGR ${pct(r.before.m.cagr, 2, true)} → ${pct(r.after.m.cagr, 2, true)}): ${holdText(r.before.m.sharpe, r.after.m.sharpe)}` : '';

/** The short summary: four lines, numbers first. */
function ccSummary(D) {
  const [b, t1] = [D.runs[0], D.runs[1]], L = [], rk = D.res.rank;
  if (!t1) return [`No setting traded often enough to rank (needs ${rk.min_trades}+ trades, ${fx(rk.per_year, 1)} a year, on the selection dates).`];
  const bookRank = D.top.findIndex(t => sameP(t.p, D.def));
  L.push(bookRank >= 0 ? `Chosen on ${D.selectionLabel || 'All history'} (${D.res.from.slice(0, 4)}–${D.res.to.slice(0, 4)}): the book settings rank #${bookRank + 1} of ${n0(rk.eligible)} distinct settings.`
    : `Chosen on ${D.selectionLabel || 'All history'} (${D.res.from.slice(0, 4)}–${D.res.to.slice(0, 4)}): #1 changes ${dSet(t1.p, D.def)}, average percentile ${fx(t1.c.score, 1)} on the five ranking stats (weakest ${fx(t1.c.weakest, 1)}).`);
  L.push(`${PERIOD_NAME[D.kind]}: #1 CAGR ${pct(t1.m.cagr, 2, true)} vs book ${pct(b.m.cagr, 2, true)}, Sharpe ${fx(t1.m.sharpe)} vs ${fx(b.m.sharpe)}, max DD ${pct(t1.m.max_dd, 1)} vs ${pct(b.m.max_dd, 1)}, expectancy ${fx(t1.m.expectancy_ratio)} vs ${fx(b.m.expectancy_ratio)}.`);
  if (D.noBefore) L.push(`No before-book test: ${D.noBefore}.`);
  else { if (baText(t1, D.pub)) L.push(`#1 ${baText(t1, D.pub)}.`); if (baText(b, D.pub)) L.push(`Book ${baText(b, D.pub)}.`); }
  return L;
}

function conclusionHtml(D, analyst = null) {
  const r = D.res, rk = r.rank, def = D.def, d0 = D.runs[0].d;
  const head = `<div class="cc-head"><span class="cc-chip">${esc(D.selectionLabel || 'All history')} · ${r.from} to ${r.to}</span><span class="cc-chip">${n0(r.combos)} settings tested</span><span class="cc-chip">${n0(rk.eligible)} distinct ranked (${rk.min_trades}+ trades, ${fx(rk.per_year, 1)} a year)</span><span class="cc-chip">Showing ${esc(PERIOD_NAME[D.kind])} · ${d0.dates[0]} to ${d0.dates[d0.dates.length - 1]}</span><span class="cc-chip">${esc(D.region)} · ${esc(D.cost)} fees + ${esc(D.slippage??0)} slippage bps per side</span></div>`;
  const sum = `<ul class="cc-sum">${(analyst ? analyst.summary : ccSummary(D)).map(t => `<li>${esc(t)}</li>`).join('')}</ul>`;
  const tbl = `<div class="tlog"><table class="tbl cc-tbl"><thead><tr><th></th><th>Settings</th>${statHead()}<th class="n">Score</th><th></th></tr></thead><tbody>
    ${D.runs.map((x, k) => `<tr class="${x.book ? 'book' : k === 1 ? 'pick' : ''}"><td>${x.name}</td><td class="cset">${x.book ? '<span class="muted">as printed</span>' : setLine(x.p, def)}</td>${statCells(x.m)}<td class="n">${x.c ? fx(x.c.score, 1) : '–'}</td><td class="n">${x.book || sameP(x.p, def) ? '' : applyLink(x.p)}</td></tr>`).join('')}</tbody></table></div>`;
  const BA = CC_STATS;
  const side = (x, k) => x?.m ? BA.map(([f, , fm, sg], j) => `<td class="n ${f === 'max_dd' ? 'neg' : sg ? cls(x.m[f]) : ''}${k === 'a' && !j ? ' ba-split' : ''}">${fm(x.m[f])}</td>`).join('') : `<td class="n muted${k === 'a' ? ' ba-split' : ''}" colspan="${BA.length}">${esc(k === 'b' && D.noBefore ? 'no data before the book' : (x?.error || 'no trades').replace(/^data gap.*/, 'data gap').slice(0, 60))}</td>`;
  const ba = `<h4 class="cc-h">Same settings before and after the book (${D.pub})</h4><div class="tlog"><table class="tbl cc-tbl cc-ba-t"><thead><tr><th rowspan="2"></th><th class="n grp" colspan="${BA.length}">Before ${D.pub}</th><th class="n grp ba-split" colspan="${BA.length}">${D.pub} onward</th></tr>
    <tr>${BA.map(([, t]) => `<th class="n">${t}</th>`).join('')}${BA.map(([, t], j) => `<th class="n${j ? '' : ' ba-split'}">${t}</th>`).join('')}</tr></thead><tbody>
    ${D.runs.map((x, k) => `<tr class="${x.book ? 'book' : k === 1 ? 'pick' : ''}"><td>${x.name}</td>${side(x.before, 'b')}${side(x.after, 'a')}</tr>`).join('')}</tbody></table></div>
    <p class="cap">These settings were selected on ${esc(D.selectionLabel || 'All history')} (${r.from} to ${r.to}). Before/after results are descriptive; the split uses January 1 of the publication year, not a confirmed publication day. It is not an untouched holdout.${D.noBefore ? ` ${esc(D.noBefore)}.` : ''}</p>`;
  return `${head}${sum}${tbl}<p class="cap">Score = average percentile on CAGR, Sharpe, monthly thickness, expectancy ratio and CAR/MDD on the selection dates. Indistinguishable rounded summary outcomes count once; this does not prove identical trades. #1 is highlighted; changed values are in colour; Apply loads that setting on this page.</p>${ba}
    <div class="cc-actions"><button class="dbtn sm" id="cc-more" aria-expanded="false" aria-controls="cc-detail">Detailed stats</button><button class="dbtn sm" id="cc-xlsx">Download Excel</button></div>
    <div id="cc-detail" class="cc-detail" hidden></div>`;
}

function gptConclusionHtml(D, A = VaultAnalyst.analyze(D)) {
  return conclusionHtml(D,A).replaceAll('cc-more','gpt-more').replaceAll('cc-detail','gpt-detail').replaceAll('cc-xlsx','gpt-xlsx')
    + A.candidates.map(c=>`<div class="gpt-finding"><h4 class="cc-h">${esc(c.name)} · analyst assessment</h4><ul class="cc-sum">${[c.summary[0],c.summary[3],c.summary[4],...c.flags].map(t=>`<li>${esc(t)}</li>`).join('')}</ul></div>`).join('');
}
function gptDetailHtml(D,A) {
  const metricFormat=(k,v)=>CC_STATS.find(s=>s[0]===k)[2](v);
  return detailHtml(D)+A.candidates.map(c=>`<h4 class="cc-h">${esc(c.name)} · change from book on evaluation dates</h4><div class="tlog"><table class="tbl cc-tbl"><thead><tr><th>Statistic</th><th class="n">Book</th><th class="n">Candidate</th><th class="n">Difference</th><th>Assessment</th></tr></thead><tbody>${c.comparisons.map(m=>`<tr><td>${esc(m.label)}</td><td class="n">${metricFormat(m.key,m.reference)}</td><td class="n">${metricFormat(m.key,m.value)}</td><td class="n">${m.delta==null?'–':/cagr|net_profit|max_dd|win_rate|thickness/.test(m.key)?fx(m.delta*100,2)+' pp':fx(m.delta,2)}</td><td>${esc(m.assessment)}</td></tr>`).join('')}</tbody></table></div><p class="cap">${esc(c.neighborhood.note)}</p>${c.neighborhood.available?`<div class="tlog"><table class="tbl"><thead><tr><th>One-step sensitivity · ${c.neighborhood.inspected} neighbors</th><th>Candidate</th><th>Neighbor minimum</th><th>Neighbor maximum</th></tr></thead><tbody>${c.neighborhood.metrics.map(m=>`<tr><td>${esc(m.label)}</td><td>${metricFormat(m.key,m.value)}</td><td>${metricFormat(m.key,m.min)}</td><td>${metricFormat(m.key,m.max)}</td></tr>`).join('')}</tbody></table></div>`:''}`).join('')
    +gptRecordsHtml(D)+`<h4 class="cc-h">Research limits</h4><ul>${A.limits.map(t=>`<li>${esc(t)}</li>`).join('')}</ul><p class="cap">Monthly thickness = average closed-trade return / average calendar holding days × 30. It is not observed monthly profit. MaxDD and CAR/MDD use the all-history peak; the underwater display resets its peak each year. Formula definitions, monthly results and closed-trade records are in the workbook.</p>`;
}
function gptRecordsHtml(D) {
  return D.runs.map(run=>{
    const months=OptimCore.monthly(run.d.dates,run.d.equity),years=[...new Set(Object.keys(months).map(k=>k.slice(0,4)))],trades=run.d.trades.filter(t=>!t.open);
    return `<h4 class="cc-h" data-gpt-monthly>${esc(run.name)} · month-on-month returns</h4><div class="tlog"><table class="tbl cc-tbl"><thead><tr><th>Year</th>${Array.from({length:12},(_,i)=>`<th class="n">${i+1}</th>`).join('')}</tr></thead><tbody>${years.map(y=>`<tr><td>${y}</td>${Array.from({length:12},(_,i)=>{const v=months[y+'-'+String(i+1).padStart(2,'0')];return `<td class="n ${cls(v)}">${v==null?'–':pct(v,2,true)}</td>`;}).join('')}</tr>`).join('')}</tbody></table></div><details data-gpt-trades><summary>${esc(run.name)} · closed trades (${trades.length})</summary><p class="cap">First ${Math.min(100,trades.length)} closed trades shown. All closed trades and verification formulas are in the workbook.</p><div class="tlog"><table class="tbl"><thead><tr><th>Entry</th><th>Exit</th><th class="n">Return after modeled costs</th><th>Exit reason</th></tr></thead><tbody>${trades.slice(0,100).map(t=>`<tr><td>${esc(t.entry)}</td><td>${esc(t.exit)}</td><td class="n ${cls(t.ret)}">${pct(t.ret,3,true)}</td><td>${esc(t.reason)}</td></tr>`).join('')}</tbody></table></div></details>`;
  }).join('');
}
function gptConclusionWire(D) {
  const body=$('#gpt-body');if(!body)return;
  const A=VaultAnalyst.analyze(D);body.innerHTML=gptConclusionHtml(D,A);
  body.onclick=e=>{const b=e.target.closest('[data-apply]');if(b)go(R.path,{...S,p:b.dataset.apply});};
  $('#gpt-more').onclick=()=>{const det=$('#gpt-detail'),open=det.hidden;det.hidden=!open;$('#gpt-more').setAttribute('aria-expanded',String(open));$('#gpt-more').textContent=open?'Hide detailed stats':'Detailed stats';if(open&&!det.innerHTML)det.innerHTML=gptDetailHtml(D,A);};
  $('#gpt-xlsx').onclick=async()=>{const b=$('#gpt-xlsx');b.disabled=true;b.textContent='Building Excel…';try{await exportXlsx(D,true);b.textContent='Download GPT Excel';}catch(e){b.textContent='Excel failed: '+e.message;}finally{b.disabled=false;}};
}

function ratioToLeader(res,c,s) {
  const key=RANK_KEYS[s],g=(res.rank.groups||[]).find(g=>(g.stat==='mar'?'car_mdd':g.stat)===key);
  if(!g?.rows.length)return null;
  const leader=rowOf(res,rowAt(res,g.rows[0])).m[key],value=rowOf(res,rowAt(res,c.row)).m[key];
  return Number.isFinite(leader)&&leader>0&&Number.isFinite(value)?value/leader:null;
}

function conclusionWire(D) {
  const body = $('#cc-body');
  body.onclick = e => { const b = e.target.closest('[data-apply]'); if (b) go(R.path, { ...S, p: b.dataset.apply }); };
  if ($('#cc-more')) $('#cc-more').onclick = () => { const det = $('#cc-detail'), open = det.hidden; det.hidden = !open; $('#cc-more').setAttribute('aria-expanded', open); $('#cc-more').textContent = open ? 'Hide detailed stats' : 'Detailed stats';
    if (open && !det.innerHTML) det.innerHTML = detailHtml(D); if (open) anim(det, { opacity: [0, 1], transform: ['translateY(-4px)', 'translateY(0)'] }, { duration: .3 }); };
  if ($('#cc-xlsx')) $('#cc-xlsx').onclick = async () => { const b = $('#cc-xlsx'); b.disabled = true; b.textContent = 'Building Excel…';
    try { await exportXlsx(D); b.textContent = 'Download Excel'; } catch (err) { b.textContent = 'Excel failed: ' + err.message; } finally { b.disabled = false; } };
}

// ---------------------------------------------------------------- detailed stats: equity, underwater (peak reset each year), full table, method
const RUN_COL = ['var(--bh)', 'var(--chart)', 'var(--blue)', '#8a6a2f'];
/** Drawdown from a peak that restarts every calendar year at the previous year-end equity. */
function ddYearly(dates, eq) { let peak = 1, y = dates[0].slice(0, 4); return eq.map((v, i) => { const yy = dates[i].slice(0, 4); if (yy !== y) { y = yy; peak = eq[i - 1]; } peak = Math.max(peak, v); return v / peak - 1; }); }
function lineSvg(dates, series, { h = 300, fmt = v => v.toFixed(2), log = false } = {}) {
  const W = 1240, P = { l: 58, r: 12, t: 10, b: 24 }, n = dates.length, step = Math.max(1, Math.floor(n / 600)), idx = []; for (let i = 0; i < n; i += step) idx.push(i); if (idx[idx.length - 1] !== n - 1) idx.push(n - 1);
  const tr = v => log ? Math.log(Math.max(v, 1e-6)) : v, all = series.flatMap(s => idx.map(i => tr(s.vals[i]))).filter(isFinite), lo = Math.min(...all), hi = Math.max(...all, lo + 1e-9);
  const X = i => P.l + i / (n - 1 || 1) * (W - P.l - P.r), Y = v => P.t + (hi - tr(v)) / (hi - lo) * (h - P.t - P.b);
  const ticks = [0, .25, .5, .75, 1].map(f => lo + f * (hi - lo)), years = []; let py = ''; dates.forEach((d, i) => { const y = d.slice(0, 4); if (y !== py && (i === 0 || +y % Math.max(1, Math.ceil(n / 252 / 8)) === 0)) years.push([i, y]); py = y; });
  return `<svg viewBox="0 0 ${W} ${h}" width="100%" class="cc-svg" role="img">${ticks.map(t => `<line x1="${P.l}" x2="${W - P.r}" y1="${Y(log ? Math.exp(t) : t)}" y2="${Y(log ? Math.exp(t) : t)}" stroke="var(--rule)"/><text x="${P.l - 6}" y="${Y(log ? Math.exp(t) : t) + 4}" class="evt" text-anchor="end">${fmt(log ? Math.exp(t) : t)}</text>`).join('')}
    ${years.map(([i, y]) => `<text x="${X(i)}" y="${h - 6}" class="evt" text-anchor="middle">${y}</text>`).join('')}
    ${series.map(s => `<path d="${idx.map((i, k) => `${k ? 'L' : 'M'}${X(i).toFixed(1)},${Y(s.vals[i]).toFixed(1)}`).join('')}" fill="${s.fill ? s.color : 'none'}" fill-opacity="${s.fill ? .18 : 0}" stroke="${s.color}" stroke-width="${s.w || 1.6}"/>`).join('')}</svg>
    <p class="cap">${series.map(s => `<span class="cc-key"><i style="background:${s.color}"></i>${esc(s.name)}</span>`).join(' ')}</p>`;
}
function detailHtml(D) {
  const runs = D.runs, d0 = runs[0].d, rk = D.res.rank;
  const eq = lineSvg(d0.dates, runs.map((x, k) => ({ name: x.name, color: RUN_COL[k], vals: x.d.equity, w: k === 1 ? 2.4 : 1.6 })), { fmt: v => v.toFixed(2) + '×', log: true });
  const uw = lineSvg(d0.dates, [runs[1], runs[0]].filter(Boolean).map(x => ({ name: `${x.name} (peak resets each year)`, color: x.book ? 'var(--bh)' : 'var(--red)', vals: ddYearly(x.d.dates, x.d.equity), fill: !x.book })), { h: 230, fmt: v => (v * 100).toFixed(0) + '%' });
  const years = [...new Set(d0.dates.map(d => d.slice(0, 4)))], ydd = runs.map(x => { const dd = ddYearly(x.d.dates, x.d.equity), o = {}; x.d.dates.forEach((d, i) => { const y = d.slice(0, 4); o[y] = Math.min(o[y] ?? 0, dd[i]); }); return o; });
  const yret = runs.map(x => { const o = {}; let base = 1, y = x.d.dates[0].slice(0, 4); x.d.dates.forEach((d, i) => { const yy = d.slice(0, 4); if (yy !== y) { o[y] = x.d.equity[i - 1] / base - 1; base = x.d.equity[i - 1]; y = yy; } }); o[y] = x.d.equity[x.d.equity.length - 1] / base - 1; return o; });
  const ytbl = `<div class="tlog"><table class="tbl cc-y"><thead><tr><th>Year</th>${runs.map(x => `<th class="n">${x.name} return</th><th class="n">${x.name} max DD</th>`).join('')}</tr></thead><tbody>
    ${years.map(y => `<tr><td>${y}</td>${runs.map((x, k) => `<td class="n ${cls(yret[k][y])}">${pct(yret[k][y], 1, true)}</td><td class="n neg">${pct(ydd[k][y], 1)}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;
  const more = [['net_profit', 'Net profit', v => pct(v, 2, true)], ['cagr', 'CAGR', v => pct(v, 2, true)], ['sharpe', 'Sharpe', fx], ['sortino', 'Sortino', fx], ['max_dd', 'Max DD', v => pct(v, 2)], ['max_dd_days', 'Longest DD (bars)', v => fx(v, 0)], ['car_mdd', 'CAR/MDD', fx],
    ['trades', 'Trades', v => fx(v, 0)], ['win_rate', 'Win %', v => pct(v, 2)], ['avg_trade', 'Average trade', v => pct(v, 3, true)], ['avg_win', 'Average win', v => pct(v, 3, true)], ['avg_loss', 'Average loss', v => pct(v, 3)], ['payoff', 'Payoff (win ÷ loss)', fx],
    ['profit_factor', 'Profit factor', fx], ['expectancy_ratio', 'Expectancy ratio', fx], ['avg_hold_days', 'Average holding days', v => fx(v, 1)], ['monthly_thickness', 'Monthly thickness', v => pct(v, 2, true)], ['exposure', 'Time in market', v => pct(v, 1)], ['t_stat', 't-stat of trades', fx], ['max_consec_losses', 'Most losses in a row', v => fx(v, 0)]];
  const stbl = `<table class="tbl cc-full"><thead><tr><th>Stat</th>${runs.map(x => `<th class="n">${x.name}</th>`).join('')}</tr></thead><tbody>${more.map(([k, t, f]) => `<tr><td>${t}</td>${runs.map(x => `<td class="n">${f(x.m[k])}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
  const groups = (rk.groups || []).map(g => `<h4 class="cc-h">Top 5 by ${esc(g.label)}</h4><div class="tlog"><table class="tbl cc-tbl"><thead><tr><th>Rank</th><th>Settings</th>${RANK_KEYS.map(k=>`<th class="n">${esc(RANK_LABEL[k])}</th>`).join('')}</tr></thead><tbody>${g.rows.map((i,j)=>{const x=rowOf(D.res,rowAt(D.res,i));return `<tr><td>${j+1}</td><td>${esc(dSet(x.p,D.def))}</td>${RANK_KEYS.map(k=>`<td class="n">${CC_STATS.find(s=>s[0]===k)[2](x.m[k])}</td>`).join('')}</tr>`;}).join('')}</tbody></table></div>`).join('');
  const pool = (rk.pool || []).slice(0, 25), matrix = `<div class="tlog"><table class="tbl cc-m"><thead><tr><th>#</th><th>Settings</th>${RANK_KEYS.map(k => `<th class="n">${RANK_LABEL[k]}<br><small>value · gap to leader · pct</small></th>`).join('')}<th class="n">Score</th><th class="n">Weakest</th></tr></thead><tbody>
    ${pool.map((c, k) => `<tr class="${k < 3 ? 'pick' : ''}"><td class="rk">${k + 1}</td><td class="cset">${esc(dSet(rowOf(D.res, rowAt(D.res, c.row)).p, D.def))}</td>${RANK_KEYS.map((stat,s)=>{const x=rowOf(D.res,rowAt(D.res,c.row)),g=rk.groups.find(g=>(g.stat==='mar'?'car_mdd':g.stat)===stat),leader=g?.rows.length?rowOf(D.res,rowAt(D.res,g.rows[0])).m[stat]:null,value=x.m[stat],gap=Number.isFinite(value)&&Number.isFinite(leader)?value-leader:null,fmt=CC_STATS.find(q=>q[0]===stat)[2];return `<td class="n">${fmt(value)} · ${gap==null?'–':/cagr|thick/.test(stat)?fx(gap*100,2)+' pp':fx(gap,2)} · <b>${fx(c.pct[s],0)}</b></td>`;}).join('')}<td class="n"><b>${fx(c.score, 1)}</b></td><td class="n">${fx(c.weakest, 0)}</td></tr>`).join('')}</tbody></table></div>`;
  const steps = [['Search', `${n0(D.res.combos)} settings: varied parameters over the recorded grid, with other parameters held at their recorded fixed values, each one backtested on ${D.res.from} to ${D.res.to} in Rust.`],
    ['Filter', `${n0(rk.eligible)} traded regularly enough and were ranked: an average-rate cutoff of ${fx(rk.per_year, 1)} trades a year (half the book setting’s yearly rate, kept between 1 and 5 a year), ${rk.min_trades} closed trades in this period, and never fewer than 10.`],
    ['Percentiles', 'Each of the five stats (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD) was turned into a percentile among the ranked settings, 100 = best.'],
    ['Groups', 'The top 5 settings by each stat form five groups (below). Together they are the candidates.'],
    ['Cross-compare', 'Each candidate is compared with every group’s leader on that group’s stat: its value as a share of a positive leader’s, and its percentile. Nonpositive leader ratios are withheld; raw stats remain available.'],
    ['Top 3', 'The three candidates with the highest average percentile are selected. Ties use the higher weakest percentile, then CAGR; a high average can still conceal a weak individual metric.']];
  return `<h4 class="cc-h">Equity, growth of 1 (log scale)</h4>${eq}
    <h4 class="cc-h">Underwater: drawdown with the peak reset each calendar year</h4>${uw}
    <div class="cc-two"><div><h4 class="cc-h">All stats</h4>${stbl}</div><div><h4 class="cc-h">Year by year</h4>${ytbl}</div></div>
    <h4 class="cc-h">How the top 3 were chosen</h4><ol class="evsteps">${steps.map(([h, t]) => `<li><b>${h}</b><span>${t}</span></li>`).join('')}</ol>
    ${groups}<h4 class="cc-h">Cross-comparison of the candidates</h4>${matrix}`;
}

// ---------------------------------------------------------------- Excel (xlsx-js-style: SheetJS with cell styles, loaded on first use)
const loadXlsx = () => window.XLSX ? Promise.resolve() : new Promise((ok, no) => { const s = document.createElement('script'); s.src = 'vendor/xlsx-js-style-1.2.0.min.js'; s.onload = ok; s.onerror = () => no(new Error('could not load the Excel library')); document.head.appendChild(s); });
// minimal palette: black text, dark blue for titles and headers, green / red only to sign numbers; no fills
const XC = { teal: '1F3864', tealDark: '1F3864', soft: 'FFFFFF', ink: '000000', ink2: '000000', ink3: '595959', rule: 'BFBFBF', zebra: 'FFFFFF', green: '1E7B34', red: 'C00000', white: 'FFFFFF', best: 'FFFFFF' };
const RUN_FILL = ['FFFFFF', 'FFFFFF', 'FFFFFF', 'FFFFFF'];
const xFont = (o = {}) => ({ name: 'Calibri', sz: 11, color: { rgb: XC.ink }, ...o });
const xBorder = rgb => ({ top: { style: 'thin', color: { rgb } }, bottom: { style: 'thin', color: { rgb } }, left: { style: 'thin', color: { rgb } }, right: { style: 'thin', color: { rgb } } });
const XS = {
  title: { font: xFont({ sz: 20, bold: true, color: { rgb: XC.teal } }) },
  sub: { font: xFont({ sz: 11, color: { rgb: XC.ink2 } }) },
  h2: { font: xFont({ sz: 13, bold: true, color: { rgb: XC.tealDark } }), border: { bottom: { style: 'medium', color: { rgb: XC.teal } } } },
  th: { font: xFont({ bold: true, color: { rgb: XC.white } }), fill: { fgColor: { rgb: XC.teal } }, alignment: { horizontal: 'center', vertical: 'center', wrapText: true }, border: xBorder(XC.tealDark) },
  thL: { font: xFont({ bold: true, color: { rgb: XC.white } }), fill: { fgColor: { rgb: XC.teal } }, alignment: { horizontal: 'left', vertical: 'center' }, border: xBorder(XC.tealDark) },
  key: { font: xFont({ bold: true }), border: xBorder(XC.rule), alignment: { vertical: 'top' } },
  val: { font: xFont(), border: xBorder(XC.rule), alignment: { horizontal: 'left', vertical: 'top', wrapText: true } },
  bullet: { font: xFont(), alignment: { wrapText: true, vertical: 'top', indent: 1 }, border: { left: { style: 'thick', color: { rgb: XC.teal } } } },
  muted: { font: xFont({ sz: 10, italic: true, color: { rgb: XC.ink3 } }), alignment: { wrapText: true, vertical: 'top' } },
};
const xCell = (z, v, zebra, signed) => ({ font: xFont({ color: { rgb: signed && typeof v === 'number' && isFinite(v) && v !== 0 ? (v > 0 ? XC.green : XC.red) : XC.ink } }), fill: { fgColor: { rgb: zebra ? XC.zebra : XC.white } }, border: xBorder(XC.rule), numFmt: z, alignment: { horizontal: typeof v === 'number' ? 'right' : 'left', vertical: 'center' } });
// light heatmap: pale green for gains, pale red for losses, a little deeper for larger moves (never dark)
const heatFill = (v, cap) => { if (typeof v !== 'number' || !isFinite(v) || !cap || Math.abs(v) < 1e-12) return XC.white; const t = .12 + Math.min(1, Math.abs(v) / cap) * .28, [r, g, b] = v > 0 ? [99, 190, 123] : [248, 105, 107]; const mix = c => Math.round(255 + (c - 255) * t).toString(16).padStart(2, '0'); return (mix(r) + mix(g) + mix(b)).toUpperCase(); };
const xDate = s => (Date.UTC(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10)) - Date.UTC(1899, 11, 30)) / 864e5; // Excel serial day
/** A worksheet built cell by cell, so every cell can carry a style. */
function xSheet() {
  const ws = {}, merges = [], rows = []; let maxR = 0, maxC = 0;
  const put = (r, c, v, s = {}) => {
    let cell;
    if (v && typeof v === 'object' && v.f) cell = { t: 'n', f: v.f };
    else if (typeof v === 'number') cell = isFinite(v) ? { t: 'n', v } : { t: 's', v: '' };
    else cell = { t: 's', v: v == null ? '' : String(v) };
    cell.s = s; if (s.numFmt) cell.z = s.numFmt;
    ws[XLSX.utils.encode_cell({ r, c })] = cell; maxR = Math.max(maxR, r); maxC = Math.max(maxC, c);
  };
  return { ws, put, merge: (r, c1, c2, r2 = r) => merges.push({ s: { r, c: c1 }, e: { r: r2, c: c2 } }), height: (r, h) => { rows[r] = { hpt: h }; },
    done(cols, filter, freeze) { ws['!ref'] = XLSX.utils.encode_range({ s: { r: 0, c: 0 }, e: { r: maxR, c: maxC } }); ws['!merges'] = merges; ws['!rows'] = rows; ws['!cols'] = cols.map(w => ({ wch: w })); if (filter) ws['!autofilter'] = { ref: filter }; if (freeze) ws['!freeze'] = freeze; return ws; } };
}
function xTable(sh, r0, c0, head, rows, fmts = [], signed = [], heat = null) {
  head.forEach((h, j) => sh.put(r0, c0 + j, h, j ? XS.th : XS.thL)); sh.height(r0, 30);
  rows.forEach((row, i) => row.forEach((v, j) => { const st = xCell(fmts[j], v, i % 2 === 1, signed[j]); if (heat && heat[j]) { st.fill = { fgColor: { rgb: heatFill(v, heat[j]) } }; if (v === 0) st.font = xFont({ color: { rgb: XC.ink3 } }); } sh.put(r0 + 1 + i, c0 + j, v, st); }));
  return r0 + rows.length + 2;
}
const xH2 = (sh, r, t, w) => { sh.put(r, 0, t, XS.h2); for (let c = 1; c <= w; c++) sh.put(r, c, '', XS.h2); };

/**
 * Workbook: Summary (book + top 3, coloured), Formulas (each stat recomputed with Excel formulas from the trade and
 * daily sheets, next to the engine value), Method (groups + cross-comparison), MoM, Drawdown (peak reset each year),
 * Daily (equity, return, peaks, drawdowns as formulas) and one Trades sheet per run.
 */
async function exportFullGrid(res,v,tk) {
  const context={region:S.region,cost:Number(S.cost||0),slippage:Number(S.slippage||0),rollover:Number(S.rollover||0)};
  const table=VaultGridExport.fullGrid(res,OPT.schemaSync[v.id]||[]);
  await loadXlsx();
  const X=XLSX.utils, wb=X.book_new(), sh=X.aoa_to_sheet([table.header,...table.rows]);
  sh['!autofilter']={ref:X.encode_range({s:{r:0,c:0},e:{r:table.rows.length,c:table.header.length-1}})};
  sh['!cols']=table.header.map(t=>({wch:Math.max(14,Math.min(28,t.length+2))}));
  const percent=/^(total_return|cagr|max_dd|win_rate|avg_trade|avg_net_pnl|exposure|monthly_thickness|gross_total_return|gross_avg_trade|gross_win_rate|gross_max_dd|ledger_gross_return|fee_return|slippage_return)$/;
  for(let r=1;r<=table.rows.length;r++) for(let c=1;c<=table.metricKeys.length;c++) {
    const cell=sh[X.encode_cell({r,c})];if(cell){const format=percent.test(table.metricKeys[c-1])?'0.00%':'0.00';cell.z=format;cell.s=xCell(format,cell.v,r%2===0,percent.test(table.metricKeys[c-1]));}
  }
  table.header.forEach((_,c)=>{const cell=sh[X.encode_cell({r:0,c})];if(cell)cell.s=XS.th;});
  X.book_append_sheet(wb,sh,'All optimization rows');
  X.book_append_sheet(wb,X.aoa_to_sheet([
    ['Field','Value'],['Strategy',v.id],['Method',v.name],['Market',context.region],['Instrument',tk],['From',res.from],['To',res.to],['Cost bps per side',context.cost],['Slippage bps per side',context.slippage],['Rollover bps per actual roll',context.rollover],...(res.experiment?[['Experiment',res.experiment.kind],['Source study reproduced',false],['Experiment basis',res.experiment.basis]]:[]),['Grid combinations',res.combos],['Rows exported',table.rows.length],['Layout','All metric columns first; all varied and fixed parameter columns at the end.'],['Ranking','Existing local full-history ranking; selection is not an untouched holdout test.'],['Data','No simulated prices or missing outcomes added by export. Undefined statistics are blank.']
  ]),'Run metadata');
  XLSX.writeFile(wb,`optimization-all-${v.id}-${tk.replace(/\s/g,'')}-${res.from}_${res.to}.xlsx`);
}

async function exportXlsx(D, analyst = false) {
  await loadXlsx();
  const A=analyst?VaultAnalyst.analyze(D):null;
  const { v, tk, runs, res } = D, X = XLSX.utils, wb = X.book_new(), P = '0.00%', N2 = '0.00', I = '#,##0', DF = 'yyyy-mm-dd', keys = Object.keys(D.def);
  const W = 14, names = runs.map(x => x.name === 'Book' ? 'Book' : `Top ${x.name.slice(1)}`);
  const dates = runs[0].d.dates, nd = dates.length;
  const fmtOf = k => /cagr|net_profit|max_dd|win_rate|thickness|total_return/.test(k) ? P : k === 'trades' ? I : N2;

  // ---- Summary
  const sh = xSheet(); let r = 0;
  sh.put(r, 0, `${analyst?'GPT':'Claude'}’s conclusion: ${v.family_name}${v.variant ? ' · ' + v.variant : ''}`, XS.title); sh.height(r, 30); r++;
  sh.put(r, 0, `${LABEL[tk] || tk} · showing ${PERIOD_NAME[D.kind]} ${dates[0]} to ${dates[nd - 1]} · settings chosen on ${D.selectionLabel || 'All history'} ${res.from} to ${res.to} · ${BOOK[v.book]?.title || ''} (${D.pub}) · fees ${D.cost ?? 0} + slippage ${D.slippage ?? 0} bps a side · rollover ${D.rollover ?? 0} bps per actual roll · generated ${D.made}`, XS.sub); r += 2;
  if(res.experiment) {sh.put(r,0,res.experiment.basis,XS.val);sh.merge(r,0,10);sh.height(r,64);r++;}
  xH2(sh, r, `Book vs ${analyst?'GPT':'Claude'}’s top 3`, W); r++;
  const best = CC_STATS.map(([k, , , signed]) => { const vals = runs.map(x => x.m[k]).filter(isFinite); return k === 'max_dd' ? Math.max(...vals) : k === 'trades' || k === 'win_rate' ? null : Math.max(...vals); });
  ['Run', 'Settings', ...CC_STATS.map(s => s[1]), 'Score (avg percentile)'].forEach((h, j) => sh.put(r, j, h, j ? XS.th : XS.thL)); sh.height(r, 32); r++;
  runs.forEach((x, i) => {
    const fill = { fgColor: { rgb: RUN_FILL[i] } }, base = { border: xBorder(XC.rule), fill, alignment: { vertical: 'center', wrapText: true } };
    sh.put(r, 0, names[i], { ...base, font: xFont({ bold: true }) });
    sh.put(r, 1, x.book ? 'Book settings, as printed' : keys.map(k => `${sLabel(k)} ${x.p[k]}${Math.abs(x.p[k] - D.def[k]) > 1e-9 ? ' *' : ''}`).join(' · '), { ...base, font: xFont({ sz: 10 }) });
    CC_STATS.forEach(([k, , , signed], j) => { const val = x.m[k], top = best[j] != null && val === best[j];
      sh.put(r, 2 + j, val, { ...base, numFmt: fmtOf(k), alignment: { horizontal: 'right', vertical: 'center' }, font: xFont({ bold: top, color: { rgb: signed && isFinite(val) && val !== 0 ? (val > 0 ? XC.green : XC.red) : k === 'max_dd' ? XC.red : XC.ink } }), fill: top ? { fgColor: { rgb: XC.best } } : fill }); });
    sh.put(r, 2 + CC_STATS.length, x.c ? x.c.score : '–', { ...base, numFmt: '0.0', alignment: { horizontal: 'right' } });
    sh.height(r, 34); r++;
  });
  sh.put(r, 0, 'Bold = best of the four in that column. * = differs from the book. Every stat is recomputed with formulas on the Formulas sheet.', XS.muted); sh.merge(r, 0, W); r += 2;
  xH2(sh, r, 'Summary', W); r++;
  (A ? [...A.summary,...A.candidates.flatMap(c=>c.summary.map(t=>c.name+': '+t)),...A.limits] : ccSummary(D)).forEach(t => { sh.put(r, 0, t, XS.bullet); for (let c = 1; c <= W; c++) sh.put(r, c, '', XS.bullet); sh.merge(r, 0, W); sh.height(r, Math.max(20, Math.ceil(t.length / 170) * 16 + 6)); r++; });
  r++;
  xH2(sh, r, `Same settings before and after the book (${D.pub})`, W); r++;
  if (D.noBefore) { sh.put(r, 0, `No before-book test: ${D.noBefore}.`, XS.muted); sh.merge(r, 0, W); r++; }
  const ms = ['cagr', 'net_profit', 'max_dd', 'win_rate', 'sharpe', 'monthly_thickness', 'trades', 'expectancy_ratio', 'car_mdd'], lines = [];
  runs.forEach((x, i) => [['before', `before ${D.pub}`], ['after', `${D.pub} onward`]].forEach(([k, t]) => { if (x[k]?.m) lines.push([`${names[i]} · ${t}`, x.book ? 'Book settings' : keys.map(q => `${sLabel(q)} ${x.p[q]}`).join(' · '), ...ms.map(f => x[k].m[f] ?? null)]); }));
  r = xTable(sh, r, 0, ['Run · period', 'Settings', ...ms.map(k => CC_STATS.find(s_ => s_[0] === k)[1])], lines, [null, null, ...ms.map(fmtOf)], [0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1]);
  xH2(sh, r, 'Definitions', W); r++;
  [['CAGR', 'final equity ^ (365.25 ÷ calendar days in the test) − 1'], ['Net profit', 'sum of closed-trade net returns; excludes the open position'], ['Max DD', 'largest fall of daily equity from its running (all-time) peak'], ['Win %', 'closed trades with a return above 0 ÷ closed trades'],
    ['Sharpe', 'mean daily return ÷ standard deviation of daily returns (n − 1) × √252, risk-free rate 0'], ['Monthly thickness', 'average trade return ÷ average holding days × 30 (holding days = calendar days entry to exit, a same-day trade counts 1)'],
    ['No. of trades', 'closed trades (a trade still open at the end is excluded)'], ['Expectancy ratio', '(win % × average win − loss % × |average loss|) ÷ |average loss|  (Van Tharp: expected gain per unit of average loss)'], ['CAR/MDD', 'CAGR ÷ |max DD|'],
    ['Score', 'average of the five percentiles (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD) among the settings that trade often enough (half the book’s yearly rate, 1 to 5 a year, at least 10 trades); see the Method sheet']].forEach(([k, t]) => { sh.put(r, 0, k, XS.key); sh.put(r, 1, t, XS.val); sh.merge(r, 1, W); for (let c = 2; c <= W; c++) sh.put(r, c, '', XS.val); r++; });
  X.book_append_sheet(wb, sh.done([22, 54, ...Array(W).fill(13)]), 'Summary');

  // ---- Trades per run (dates as Excel dates so holding days are a formula)
  const tName = i => `Trades ${names[i]}`.slice(0, 31), tRows = [];
  runs.forEach((x, i) => {
    const t = xSheet(), closed = x.d.trades.filter(q => !q.open);
    t.put(0, 0, `Trades · ${names[i]}`, XS.title); t.height(0, 26);
    t.put(1, 0, x.book ? 'Book settings' : keys.map(k => `${sLabel(k)} ${x.p[k]}`).join(' · '), XS.sub);
    ['#', 'Entry', 'Exit', 'Return', 'Holding days', 'Win', 'Equity after', 'Pieces', 'Avg entry', 'Exit price', 'Exit reason'].forEach((h, j) => t.put(3, j, h, j ? XS.th : XS.thL)); t.height(3, 30);
    closed.forEach((q, k) => { const rr = 4 + k, e = rr + 1, z = k % 2 === 1;
      t.put(rr, 0, k + 1, xCell('0', 1, z)); t.put(rr, 1, xDate(q.entry), xCell(DF, 1, z)); t.put(rr, 2, xDate(q.exit), xCell(DF, 1, z)); t.put(rr, 3, q.ret, xCell(P, q.ret, z, 1));
      t.put(rr, 4, { f: `MAX(1,C${e}-B${e})` }, xCell('0', 1, z)); t.put(rr, 5, { f: `IF(D${e}>0,1,0)` }, xCell('0', 1, z)); t.put(rr, 6, { f: k ? `G${e - 1}*(1+D${e})` : `1+D${e}` }, xCell('0.0000', 1, z));
      t.put(rr, 7, q.legs.length, xCell('0', 1, z)); t.put(rr, 8, q.avg_px, xCell('#,##0.00', 1, z)); t.put(rr, 9, q.exit_px, xCell('#,##0.00', 1, z)); t.put(rr, 10, q.reason, xCell(null, 's', z)); });
    tRows.push(closed.length);
    X.book_append_sheet(wb, t.done([6, 12, 12, 10, 12, 6, 12, 8, 12, 12, 40], `A4:K${4 + closed.length}`, { ySplit: 4 }), tName(i));
  });

  // ---- Daily: equity per run with daily return, all-time peak, drawdown, and the peak reset each calendar year
  const de = xSheet(), C = X.encode_col, per = 5, col0 = i => 1 + i * per; // per run: Equity, Return, Peak, DD, Year peak (+ year DD)
  de.put(0, 0, 'Date', XS.thL);
  runs.forEach((x, i) => ['Equity', 'Daily return', 'Peak', 'Drawdown', 'Yearly DD'].forEach((h, j) => de.put(0, col0(i) + j, `${names[i]} · ${h}`, XS.th)));
  const yp = i => C(col0(runs.length) + i); // hidden helper columns: the yearly-reset peak per run
  runs.forEach((x, i) => de.put(0, col0(runs.length) + i, `${names[i]} · year peak`, XS.th));
  for (let k = 0; k < nd; k++) {
    const rr = k + 1, e = rr + 1; de.put(rr, 0, xDate(dates[k]), { numFmt: DF });
    runs.forEach((x, i) => { const c = col0(i), E = C(c), R_ = C(c + 1), PK = C(c + 2), Y = yp(i);
      de.put(rr, c, x.d.equity[k], { numFmt: '0.000000' });
      de.put(rr, c + 1, { f: k ? `${E}${e}/${E}${e - 1}-1` : `${E}${e}-1` }, { numFmt: '0.0000%' });
      de.put(rr, c + 2, { f: k ? `MAX(${PK}${e - 1},${E}${e})` : `MAX(1,${E}${e})` }, { numFmt: '0.000000' });
      de.put(rr, c + 3, { f: `${E}${e}/${PK}${e}-1` }, { numFmt: '0.00%' });
      de.put(rr, col0(runs.length) + i, { f: k ? `IF(YEAR(A${e})<>YEAR(A${e - 1}),MAX(${E}${e - 1},${E}${e}),MAX(${Y}${e - 1},${E}${e}))` : `MAX(1,${E}${e})` }, { numFmt: '0.000000' });
      de.put(rr, c + 4, { f: `${E}${e}/${Y}${e}-1` }, { numFmt: '0.00%' }); });
  }
  X.book_append_sheet(wb, de.done([12, ...Array(runs.length * (per + 1)).fill(14)], null, { xSplit: 1, ySplit: 1 }), 'Daily');

  // ---- Formulas: every stat recomputed from the Trades and Daily sheets, beside the engine's number
  const fs = xSheet(); let q = 0, L = nd + 1; // last Daily row (Excel)
  fs.put(q, 0, 'Stats recomputed with Excel formulas', XS.title); fs.height(q, 28); q++;
  fs.put(q, 0, 'Each formula reads only the Trades and Daily sheets of this workbook. "Engine" is the number from the Rust API. Small differences (≤ 0.1%) come from the API rounding prices and returns to 6 decimals.', XS.muted); fs.merge(q, 0, 8); fs.height(q, 30); q += 2;
  runs.forEach((x, i) => {
    xH2(fs, q, names[i], 5); q++;
    ['Stat', 'Formula result', 'Engine', 'Difference', 'Check', 'Formula'].forEach((h, j) => fs.put(q, j, h, j ? XS.th : XS.thL)); q++;
    const T = `'${tName(i)}'`, n = tRows[i], rT = c => `${T}!${c}5:${c}${4 + Math.max(1, n)}`, c = col0(i), E = C(c), Rr = C(c + 1), D_ = C(c + 3), first = q + 1;
    const rowsF = [
      ['CAGR', `Daily!${E}${L}^(365.25/(Daily!A${L}-Daily!A2))-1`, 'cagr', P],
      ['Net profit', `SUM(${rT('D')})`, 'net_profit', P],
      ['Max DD', `MIN(Daily!${D_}2:${D_}${L})`, 'max_dd', P],
      ['Win %', `IFERROR(COUNTIF(${rT('D')},">0")/COUNT(${rT('D')}),"")`, 'win_rate', P],
      ['Sharpe', `IFERROR(AVERAGE(Daily!${Rr}2:${Rr}${L})/STDEV(Daily!${Rr}2:${Rr}${L})*SQRT(252),"")`, 'sharpe', N2],
      ['Monthly thickness', `IFERROR(AVERAGE(${rT('D')})/AVERAGE(${rT('E')})*30,"")`, 'monthly_thickness', P],
      ['No. of trades', `COUNT(${rT('D')})`, 'trades', I],
      ['Expectancy ratio', `IFERROR((B${first + 3}*IFERROR(AVERAGEIF(${rT('D')},">0"),0)-(1-B${first + 3})*ABS(IFERROR(AVERAGEIF(${rT('D')},"<=0"),0)))/ABS(IFERROR(AVERAGEIF(${rT('D')},"<=0"),0)),"")`, 'expectancy_ratio', N2],
      ['CAR/MDD', `IFERROR(B${first}/ABS(B${first + 2}),"")`, 'car_mdd', N2]];
    rowsF.forEach(([t, f, k, z], j) => { const e = q + 1, zb = j % 2 === 1;
      fs.put(q, 0, t, { ...XS.key }); fs.put(q, 1, { f }, xCell(z, 1, zb)); fs.put(q, 2, x.m[k], xCell(z, 1, zb));
      fs.put(q, 3, { f: `IF(AND(ISNUMBER(B${e}),ISNUMBER(C${e})),B${e}-C${e},"")` }, xCell('0.000000', 1, zb));
      fs.put(q, 4, { f: `IF(AND(ISNUMBER(B${e}),ISNUMBER(C${e})),IF(ABS(B${e}-C${e})<=0.001*MAX(1,ABS(C${e})),"match","differs"),"unavailable")` }, { ...xCell(null, 's', zb), font: xFont({ bold: true, color: { rgb: XC.green } }) });
      fs.put(q, 5, '=' + f, { font: xFont({ sz: 9, color: { rgb: XC.ink3 } }) }); q++; });
    q++;
  });
  X.book_append_sheet(wb, fs.done([20, 16, 14, 14, 10, 120]), 'Formulas');

  // ---- Method: the groups and the cross-comparison behind the top 3
  const rk = res.rank, ms_ = xSheet(); let m = 0;
  ms_.put(m, 0, 'How the top 3 were chosen', XS.title); ms_.height(m, 28); m += 2;
  [`1. Search: ${n0(res.combos)} settings (every parameter over its range) backtested on ${res.from} to ${res.to}.`, `2. Filter: ${n0(rk.eligible)} settings are ranked: at least ${fx(rk.per_year, 1)} trades a year (half the book setting’s yearly rate, between 1 and 5 a year), i.e. ${rk.min_trades} closed trades here, never fewer than 10.`,
    '3. Percentiles: CAGR, Sharpe, monthly thickness, expectancy ratio and CAR/MDD are each turned into a percentile (0–100, 100 = best) among the ranked settings.',
    '4. Groups: the top 5 by each stat form five groups; together they are the candidates.', '5. Cross-compare: each candidate is set against each group leader on that group’s stat (ratio only when the leader is positive, and percentile).',
    '6. Top 3: highest average percentile across the five stats; ties go to the higher weakest percentile, then CAGR.'].forEach(t => { ms_.put(m, 0, t, XS.val); ms_.merge(m, 0, 9); m++; });
  m++;
  const cand = i => rowOf(res, rowAt(res, i));
  (rk.groups || []).forEach(g => { const k = g.stat === 'mar' ? 'car_mdd' : g.stat; xH2(ms_, m, `Top 5 by ${g.label}`, 9); m++;
    m = xTable(ms_, m, 0, ['#', 'Settings', ...RANK_KEYS.map(x => RANK_LABEL[x])], g.rows.map((i, j) => { const c = cand(i); return [j + 1, keys.map(x => `${sLabel(x)} ${c.p[x]}`).join(' · '), ...RANK_KEYS.map(x => c.m[x])]; }),
      ['0', null, P, N2, P, N2, N2], [0, 0, 1, 1, 1, 1, 1]); });
  xH2(ms_, m, 'Cross-comparison of every candidate (value as % of the group leader · percentile)', 9); m++;
  m = xTable(ms_, m, 0, ['Rank', 'Settings', ...RANK_KEYS.flatMap(x => [`${RANK_LABEL[x]} % of leader`, `${RANK_LABEL[x]} percentile`]), 'Score', 'Weakest'],
    (rk.pool || []).map((c, j) => [j + 1, keys.map(x => `${sLabel(x)} ${cand(c.row).p[x]}`).join(' · '), ...RANK_KEYS.flatMap((x, s) => [ratioToLeader(res,c,s), c.pct[s]]), c.score, c.weakest]),
    ['0', null, ...RANK_KEYS.flatMap(() => [P, '0.0']), '0.0', '0.0'], [], [0, 0, ...RANK_KEYS.flatMap(() => [0, 100]), 100, 100]);
  X.book_append_sheet(wb, ms_.done([8, 60, ...Array(12).fill(14)]), 'Method');

  // ---- Month on month: one grid per top-3 setting. Years down, months across; Year = compounded year return, Avg = average month;
  // the Avg row averages each month over all years. Totals and averages are Excel formulas (blank = no data that month).
  const mo = xSheet(); let o = 0;
  mo.put(o, 0, 'Month-on-month P&L', XS.title); mo.height(o, 28); o++;
  mo.put(o, 0, 'Month-end equity ÷ previous month-end equity − 1. Light green = gain, light red = loss (deeper = larger); 0.00% = no position that month. Year = the 12 months compounded; Avg = average month of that year; the Avg row = average of that month over all years.', XS.muted); mo.merge(o, 0, 14); mo.height(o, 30); o += 2;
  const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const numSt = (v, bold, cap) => ({ fill: { fgColor: { rgb: cap ? heatFill(v, cap) : XC.white } }, font: xFont({ bold, color: { rgb: typeof v === 'number' && isFinite(v) && Math.abs(v) > 1e-12 ? (v > 0 ? XC.green : XC.red) : XC.ink } }), numFmt: P, border: xBorder(XC.rule), alignment: { horizontal: 'right' } });
  const avg = a => { const b = a.filter(x => x != null && isFinite(x)); return b.length ? b.reduce((p, q) => p + q, 0) / b.length : null; };
  runs.forEach((x, i) => { if (x.book) return;
    const m = OptimCore.monthly(x.d.dates, x.d.equity), years = [...new Set(Object.keys(m).map(k => k.slice(0, 4)))];
    xH2(mo, o, `${names[i]} · ${keys.map(k => `${sLabel(k)} ${x.p[k]}`).join(' · ')}`, 14); o++;
    ['Year', ...MON, 'Year', 'Avg month'].forEach((h, j) => mo.put(o, j, h, j ? XS.th : XS.thL)); o++;
    const top = o + 1, rowVals = [], capM = Math.max(1e-9, ...Object.values(m).filter(isFinite).map(Math.abs)), capY = Math.max(1e-9, ...years.map(y => Math.abs(MON.reduce((p, _, k) => p * (1 + (m[`${y}-${String(k + 1).padStart(2, '0')}`] ?? 0)), 1) - 1)));
    years.forEach(y => { const e = o + 1, vals = MON.map((_, k) => m[`${y}-${String(k + 1).padStart(2, '0')}`] ?? null); rowVals.push(vals);
      mo.put(o, 0, +y, { font: xFont({ bold: true }), border: xBorder(XC.rule) });
      vals.forEach((v, k) => mo.put(o, k + 1, v == null ? '' : v, numSt(v, false, capM)));
      const yr = vals.reduce((p, v) => v == null ? p : p * (1 + v), 1) - 1, am = avg(vals);
      mo.put(o, 13, { f: `SUMPRODUCT(PRODUCT(1+B${e}:M${e}))-1` }, numSt(yr, true, capY));
      mo.put(o, 14, { f: `AVERAGE(B${e}:M${e})` }, numSt(am, false, capM)); o++; });
    const bot = o, col = j => X.encode_col(j);
    mo.put(o, 0, 'Avg', { font: xFont({ bold: true, color: { rgb: XC.teal } }), border: { ...xBorder(XC.rule), top: { style: 'medium', color: { rgb: XC.teal } } } });
    for (let j = 1; j <= 14; j++) { const v = j <= 12 ? avg(rowVals.map(r_ => r_[j - 1])) : j === 13 ? avg(rowVals.map(r_ => r_.reduce((p, q) => q == null ? p : p * (1 + q), 1) - 1)) : avg(rowVals.map(avg));
      mo.put(o, j, { f: `AVERAGE(${col(j)}${top}:${col(j)}${bot})` }, { ...numSt(v, true), border: { ...xBorder(XC.rule), top: { style: 'medium', color: { rgb: XC.teal } } } }); }
    o += 3; });
  X.book_append_sheet(wb, mo.done([10, ...Array(12).fill(9), 10, 11]), 'MoM');

  // ---- Drawdown by year (peak reset each calendar year) and the calendar-year returns
  const yrs = [...new Set(dates.map(d => d.slice(0, 4)))], ydd = runs.map(x => { const dd = ddYearly(x.d.dates, x.d.equity), w = {}; x.d.dates.forEach((d, i) => { const y = d.slice(0, 4); w[y] = Math.min(w[y] ?? 0, dd[i]); }); return w; });
  const yret = runs.map(x => { const w = {}; let base = 1, y = x.d.dates[0].slice(0, 4); x.d.dates.forEach((d, i) => { const yy = d.slice(0, 4); if (yy !== y) { w[y] = x.d.equity[i - 1] / base - 1; base = x.d.equity[i - 1]; y = yy; } }); w[y] = x.d.equity[x.d.equity.length - 1] / base - 1; return w; });
  const dd = xSheet(); let z = 0; dd.put(z, 0, 'Drawdown by year (peak reset each calendar year)', XS.title); dd.height(z, 28); z += 2;
  const capD = Math.max(...ydd.flatMap(w => Object.values(w)).map(Math.abs), 1e-9), capY = Math.max(...yret.flatMap(w => Object.values(w)).map(Math.abs), 1e-9);
  xTable(dd, z, 0, ['Year', ...names.flatMap(n => [`${n} return`, `${n} max DD`])], yrs.map(y => [y, ...runs.flatMap((_, i) => [yret[i][y] ?? null, ydd[i][y] ?? null])]),
    [null, ...runs.flatMap(() => [P, P])], [], [0, ...runs.flatMap(() => [capY, capD])]);
  X.book_append_sheet(wb, dd.done([8, ...Array(runs.length * 2).fill(14)]), 'Drawdown');

  // Sheet order: Summary, Formulas, Method, MoM, Drawdown, Trades…, Daily
  const order = ['Summary', 'Formulas', 'Method', 'MoM', 'Drawdown', ...runs.map((_, i) => tName(i)), 'Daily'];
  wb.SheetNames = order.filter(n => wb.SheetNames.includes(n));
  XLSX.writeFile(wb, `${analyst?'gpt-':''}conclusion-${v.id}-${tk.replace(/\s/g, '')}-${dates[0]}_${dates[nd - 1]}.xlsx`);
}
