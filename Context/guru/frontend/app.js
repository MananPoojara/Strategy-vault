// Strategy Vault — Guru's UI. Every number comes from the Rust API; this file only renders.
// API base: same host that served the page, port 8090 (works from any PC on the LAN).
// Override with ?api=http://host:port
const API = new URLSearchParams(location.search).get('api') || `http://${location.hostname || 'localhost'}:8090`;
const LWC = window.LightweightCharts;

const $ = s => document.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const pct = (v, d = 2, sign = false) => v == null ? '–' : (sign && v > 0 ? '+' : '') + (v * 100).toFixed(d) + '%';
const fx = (v, d = 2) => v == null ? '–' : v.toFixed(d);
const neg = v => (v != null && v < 0 ? 'neg' : '');
const api = async u => { const r = await fetch(API + u); if (!r.ok) throw new Error(await r.text()); return r.json(); };
// Steel Deposit palette. 'blue' = the one data accent (strategy / gains), 'orange' = buy & hold (slate).
const C = { blue: '#2f5a8f', orange: '#8a90a0', red: '#c43c3c', ink: '#090909', ink2: '#636466', ink3: '#8b8d93', rule: '#b0b0b1', rule2: '#dcdcdd' };
const INDEX_LABEL = { 'NIFTY 50': 'Nifty 50', NIFTYBANK: 'Bank Nifty', NIFTYMIDCAP100: 'Midcap 100', NIFTYIT: 'Nifty IT' };
const label = t => INDEX_LABEL[t] || t;

const S = { gurus: [], guru: null, catalogue: [], summary: {}, tickers: [], ticker: 'NIFTY 50', sel: null, detail: null, charts: [] };

// ------------------------------------------------------------------ boot & routing

async function init() {
  const [tickers, catalogue, gurus] = await Promise.all([api('/api/tickers'), api('/api/guru/connors/strategies'), api('/api/gurus')]);
  S.tickers = tickers; S.catalogue = catalogue; S.gurus = gurus; S.guru = gurus[0];
  $('#runbar').onsubmit = e => { e.preventDefault(); run(); };
  $('#q').oninput = renderList;
  $('#sort').onchange = renderList;
  document.querySelectorAll('.tab').forEach(b => b.onclick = () => showTab(b.dataset.tab));
  renderTickers();
  setRange();
  window.addEventListener('hashchange', route);
  await run(true);
  route();
}

function route() {
  const [, guru, strat] = location.hash.split('/'); // "#/connors/tps_long"
  const onGuru = guru === S.guru.id;
  $('#view-gallery').hidden = onGuru;
  $('#view-guru').hidden = !onGuru;
  if (!onGuru) { renderGallery(); return; }
  renderGuruHead();
  const id = strat || S.sel || 'tps_long';
  if (id !== S.sel || !S.detail) openStrategy(id, false);
}

function renderTickers() {
  $('#tickers').innerHTML = S.tickers.map(t => `<button type="button" role="radio" data-t="${esc(t.ticker)}" class="${t.ticker === S.ticker ? 'on' : ''}" aria-checked="${t.ticker === S.ticker}">${esc(label(t.ticker))}${t.has_volume ? '' : '<small>no volume</small>'}</button>`).join('');
  $('#tickers').querySelectorAll('button').forEach(b => b.onclick = () => { S.ticker = b.dataset.t; renderTickers(); setRange(); run(); });
}
function setRange() { const t = S.tickers.find(t => t.ticker === S.ticker); $('#from').value = t.first; $('#to').value = t.last; }
function params(extra = {}) {
  return new URLSearchParams({ ticker: S.ticker, from: $('#from').value, to: $('#to').value, cost: $('#cost').value || 0, ...extra });
}

async function run(silent) {
  $('#status').textContent = 'Running backtests…';
  try {
    const rows = await api('/api/guru/connors/summary?' + params());
    S.summary = Object.fromEntries(rows.map(r => [r.id, r]));
    $('#status').textContent = `${label(S.ticker)} · ${$('#from').value} to ${$('#to').value} · cost ${$('#cost').value || 0} bps`;
    renderList();
    renderGuruHead();
    if (!$('#tab-compare').hidden) renderCompare();
    if (!silent && S.sel) await openStrategy(S.sel, false);
    if (!$('#view-gallery').hidden) renderGallery();
  } catch (e) { $('#status').textContent = `Could not reach the Rust API at ${API}. Start it with: cargo run --release (${e.message})`; }
}

function showTab(name) {
  document.querySelectorAll('.tab').forEach(b => b.classList.toggle('on', b.dataset.tab === name));
  document.querySelectorAll('.panel-tab').forEach(p => p.hidden = p.id !== 'tab-' + name);
  if (name === 'compare') renderCompare();
  if (name === 'thesis') renderThesis();
  if (name === 'blocked') renderBlocked();
}

// ------------------------------------------------------------------ gallery

function initials(name) { return name.split(' ').map(w => w[0]).join(''); }

// Rule sets that the course presents as versions of one idea form ONE strategy with variants.
const fullName = c => c.variant ? `${c.family_name} · ${c.variant}` : c.family_name;
function families(liveOnly = true) {
  const out = [], by = {};
  for (const c of S.catalogue) {
    if (liveOnly && c.status !== 'live') continue;
    if (!by[c.family]) out.push(by[c.family] = { id: c.family, name: c.family_name, group: c.group.split(' · ')[0], session: c.session, variants: [] });
    by[c.family].variants.push(c);
  }
  return out;
}
const familyOf = id => families().find(f => f.variants.some(v => v.id === id));
// the variant a family row shows: the selected one, else the best Sharpe, else the first
function repVariant(f) {
  return f.variants.find(v => v.id === S.sel)
    || [...f.variants].filter(v => S.summary[v.id]?.stats?.trades).sort((a, b) => S.summary[b.id].stats.sharpe - S.summary[a.id].stats.sharpe)[0]
    || f.variants[0];
}

// Guilloché: hypotrochoid rosettes, the engraving used on banknotes and share certificates.
function guilloche() {
  const paths = [[80, 23, 60, 23], [74, 19, 48, 19], [66, 29, 40, 29]].map(([R, r, d, turns]) => {
    let p = '';
    for (let k = 0; k <= 2400; k++) {
      const t = k / 2400 * Math.PI * 2 * turns;
      const x = (R - r) * Math.cos(t) + d * Math.cos((R - r) / r * t), y = (R - r) * Math.sin(t) - d * Math.sin((R - r) / r * t);
      p += `${k ? 'L' : 'M'}${(100 + x * .7).toFixed(2)},${(100 + y * .7).toFixed(2)}`;
    }
    return `<path d="${p}"/>`;
  });
  return `<svg viewBox="0 0 200 200">${paths.join('')}</svg>`;
}
const GUILLOCHE = guilloche();

// Framed grayscale portrait; falls back to initials until img/gurus/<id>.jpg exists.
function portrait(g) {
  return `<div class="pframe"><img src="img/gurus/${g.id}.jpg" alt="${esc(g.name)}" onerror="this.remove()"><span>${esc(initials(g.name))}</span></div>`;
}

const PREVIEW = [
  { id: 'raschke', name: 'Linda Raschke', book: 'Street Smarts (1995)', tagline: 'Short-term patterns and momentum pullbacks, traded with discipline.' },
  { id: 'minervini', name: 'Mark Minervini', book: 'Trade Like a Stock Market Wizard (2013)', tagline: 'Buy leaders in stage-two uptrends at low-risk entry points.' },
  { id: 'darvas', name: 'Nicolas Darvas', book: 'How I Made $2,000,000 in the Stock Market (1960)', tagline: 'Boxes, breakouts and trailing stops.' },
  { id: 'oneil', name: "William O'Neil", book: 'How to Make Money in Stocks (1988)', tagline: 'CAN SLIM: earnings growth, leadership and market direction.' },
  { id: 'livermore', name: 'Jesse Livermore', book: 'How to Trade in Stocks (1940)', tagline: 'Pivotal points, the line of least resistance, and patience.' },
];

function renderGallery() {
  const rows = Object.values(S.summary).filter(r => r.stats && r.stats.trades >= 20);
  const best = rows.sort((a, b) => b.stats.sharpe - a.stats.sharpe)[0];
  const bestDef = best && S.catalogue.find(c => c.id === best.id);
  const strategies = families(false).length, testable = families().length;
  if (!$('#gal-guilloche').innerHTML) $('#gal-guilloche').innerHTML = GUILLOCHE;
  $('#vault-counts').innerHTML = `<div><dt>Legends</dt><dd>${S.gurus.length}</dd></div><div><dt>Strategies catalogued</dt><dd>${strategies}</dd></div><div><dt>Tested on NSE data</dt><dd>${testable}</dd></div><div><dt>Indices</dt><dd>${S.tickers.length}</dd></div>`;
  const card = (x, k, live) => `
    <${live ? `a href="#/${x.id}"` : 'div'} class="legend-card${live ? '' : ' preview'}">
      <div class="lc-top"><span class="no">Box ${String(k + 1).padStart(2, '0')}${live ? '' : ' · layout preview, not converted yet'}</span><span class="keyhole"></span></div>
      <div class="lc-body">${portrait(x)}<div><h3>${esc(x.name)}</h3><div class="book">${esc(x.book)}</div></div></div>
      <p class="lc-tag">${esc(x.tagline)}</p>
      <dl class="lc-stats">
        <div><dt>Strategies</dt><dd>${live ? strategies : '–'}</dd></div>
        <div><dt>Testable</dt><dd>${live ? testable : '–'}</dd></div>
        <div><dt>Best Sharpe${live ? ' · ' + esc(label(S.ticker)) : ''}</dt><dd>${live && best ? fx(best.stats.sharpe) : '–'}</dd></div>
      </dl>
      ${live ? `<div class="lc-foot"><span>${bestDef ? `Strongest: ${esc(fullName(bestDef))}` : ''}</span><b>Open the vault →</b></div>` : ''}
    </${live ? 'a' : 'div'}>`;
  $('#gallery').innerHTML = S.gurus.map((x, k) => card(x, k, true)).join('') +
    (S.preview ? PREVIEW.map((x, k) => card(x, S.gurus.length + k, false)).join('') : '') +
    `<div class="legend-card next"><h3>Next legend</h3><p>Each book added to <b>/Books</b> becomes a box here, with its thesis, rules and tested strategies.</p>
      <button type="button" class="btn small" id="preview-toggle">${S.preview ? 'Hide the layout preview' : 'Preview the vault with more legends'}</button></div>`;
  $('#preview-toggle').onclick = () => { S.preview = !S.preview; renderGallery(); };
}

// ------------------------------------------------------------------ legend page

function renderGuruHead() {
  const g = S.guru, live = Object.values(S.summary).filter(r => r.stats);
  $('#crumb-name').textContent = g.name;
  $('#g-name').textContent = g.name;
  $('#g-tag').textContent = g.tagline;
  $('#g-book').textContent = g.book;
  if (!$('#g-portrait').innerHTML) $('#g-portrait').innerHTML = portrait(g);
  if (!$('#g-guilloche').innerHTML) $('#g-guilloche').innerHTML = GUILLOCHE;
  const sharpe = live.filter(r => r.stats.trades >= 30).map(r => r.stats.sharpe);
  $('#g-facts').innerHTML = `
    <div><dt>Strategies in the course</dt><dd>${families(false).length}</dd></div>
    <div><dt>Rule variants tested on ${esc(label(S.ticker))}</dt><dd>${live.length}</dd></div>
    <div><dt>Variants with t ≥ 2</dt><dd>${live.filter(r => r.stats.t_stat >= 2).length}</dd></div>
    <div><dt>Best Sharpe (30+ trades)</dt><dd>${sharpe.length ? fx(Math.max(...sharpe)) : '–'}</dd></div>`;
  $('#blocked-n').textContent = families(false).length - families().length;
}

function spark(vals) {
  if (!vals || vals.length < 2) return '<svg width="64" height="28"></svg>';
  const lo = Math.min(...vals), hi = Math.max(...vals), n = vals.length;
  const pts = vals.map((v, i) => `${(i / (n - 1) * 62 + 1).toFixed(1)},${(26 - (v - lo) / ((hi - lo) || 1) * 24).toFixed(1)}`).join(' ');
  const col = vals[n - 1] >= vals[0] ? C.blue : C.red;
  return `<svg width="64" height="28" viewBox="0 0 64 28" aria-hidden="true"><polyline points="${pts}" fill="none" stroke="${col}" stroke-width="1.6" stroke-linejoin="round"/></svg>`;
}

function renderList() {
  const q = $('#q').value.trim().toLowerCase(), sort = $('#sort').value;
  let fams = families().filter(f => !q || (f.name + ' ' + f.group + ' ' + f.variants.map(v => v.variant).join(' ')).toLowerCase().includes(q));
  const stat = (f, k) => S.summary[repVariant(f).id]?.stats?.[k] ?? null;
  if (sort !== 'book') fams = [...fams].sort((a, b) => (stat(b, sort) ?? -1e9) - (stat(a, sort) ?? -1e9));
  const metricKey = sort === 'book' ? 'cagr' : sort;
  let html = '', grp = null;
  for (const f of fams) {
    if (sort === 'book' && f.group !== grp) { grp = f.group; html += `<div class="list-grp">${esc(grp)}</div>`; }
    const c = repVariant(f), r = S.summary[c.id] || {}, v = r.stats?.[metricKey], n = f.variants.length;
    const shown = !r.stats ? '–' : metricKey === 'cagr' ? pct(v, 1, true) : metricKey === 'win_rate' ? pct(v, 0) : fx(v, metricKey === 'trades' ? 0 : 2);
    const sub = r.blocked ? esc(r.blocked) : `${n > 1 ? `${n} variants · ` : ''}${c.dir === 'Short' && n === 1 ? 'Short · ' : ''}${fx(r.stats?.trades, 0)} trades · ${pct(r.stats?.win_rate, 0)} win`;
    const on = f.variants.some(x => x.id === S.sel);
    html += `<div class="li${on ? ' on' : ''}" data-id="${c.id}" data-fam="${f.id}" tabindex="0" role="button">
      <div class="li-name">${esc(f.name)}</div><div class="li-sub">${sub}</div>
      <div class="li-right"><span class="li-val ${neg(v)}">${shown}</span>${spark(r.spark)}</div></div>`;
  }
  $('#list').innerHTML = html || '<div class="empty">No strategy matches that filter.</div>';
  $('#list').querySelectorAll('.li').forEach(el => {
    el.onclick = () => openStrategy(el.dataset.id, true);
    el.onkeydown = e => { if (e.key === 'Enter') openStrategy(el.dataset.id, true); };
  });
}

// Variant switcher + side-by-side comparison of every variant in the family.
function variantPanel(c) {
  const f = familyOf(c.id);
  if (!f || f.variants.length < 2) return '';
  const row = v => {
    const r = S.summary[v.id], st = r?.stats;
    const cells = st ? `<td class="n">${fx(st.trades, 0)}</td><td class="n">${pct(st.win_rate, 1)}</td><td class="n ${neg(st.avg_trade)}">${pct(st.avg_trade, 2)}</td><td class="n ${neg(st.cagr)}">${pct(st.cagr, 2, true)}</td><td class="n ${neg(st.sharpe)}">${fx(st.sharpe)}</td><td class="n neg">${pct(st.max_dd, 1)}</td><td class="n">${fx(st.t_stat)}</td>`
      : `<td colspan="7" class="muted">${esc(r?.blocked || 'no data')}</td>`;
    return `<tr class="click${v.id === c.id ? ' sel' : ''}" data-id="${v.id}"><td>${v.id === c.id ? '▸ ' : ''}${esc(v.variant)}</td>${cells}<td class="n muted">${v.claim ? esc(v.claim.win) : '–'}</td></tr>`;
  };
  return `<div class="variants">
    <div class="v-head"><span>${f.variants.length} variants in the course</span><div class="seg vseg">${f.variants.map(v => `<button type="button" data-id="${v.id}" class="${v.id === c.id ? 'on' : ''}">${esc(v.variant.split(' · ')[0])}</button>`).join('')}</div></div>
    <div class="scroll" style="max-height:none"><table class="vtable"><thead><tr><th>Variant</th><th class="n">Trades</th><th class="n">Win</th><th class="n">Avg trade</th><th class="n">CAGR</th><th class="n">Sharpe</th><th class="n">Max DD</th><th class="n">t-stat</th><th class="n">Book win</th></tr></thead>
    <tbody>${f.variants.map(row).join('')}</tbody></table></div></div>`;
}
function wireVariants() {
  document.querySelectorAll('.variants [data-id]').forEach(el => el.onclick = () => openStrategy(el.dataset.id, true));
}

async function openStrategy(id, push) {
  const c = S.catalogue.find(x => x.id === id && x.status === 'live');
  if (!c) return;
  S.sel = id;
  if (push) history.replaceState(null, '', `#/${S.guru.id}/${id}`);
  renderList(); // the family row shows the selected variant's numbers
  showTab('strategies');
  const r = S.summary[id];
  if (r?.blocked) {
    disposeCharts();
    $('#sheet').innerHTML = `<div class="box">${titleBlock(c)}${variantPanel(c)}<p class="empty">${esc(r.blocked)}. Pick an index that has volume data (Nifty 50, Bank Nifty or Midcap 100).</p></div>`;
    wireVariants();
    return;
  }
  try { S.detail = await api('/api/guru/connors/run?' + params({ id })); }
  catch (e) { $('#sheet').innerHTML = `<div class="box empty">${esc(e.message)}</div>`; return; }
  renderSheet(c, S.detail);
}

function titleBlock(c, d) {
  const win = d ? `${d.dates[0]} to ${d.dates[d.dates.length - 1]} · ${fx(d.stats.years, 1)} years` : '';
  return `<div class="sheet-title"><div><h2>${esc(c.family_name)}</h2>${c.variant ? `<div class="vname">Variant: <b>${esc(c.variant)}</b></div>` : ''}
    <div class="meta"><span class="dirtag${c.dir === 'Short' ? ' short' : ''}">${c.dir}</span><span>${esc(c.group)}</span><span>Session ${c.session}</span><span>${esc(label(S.ticker))}</span>${win ? `<span>${win}</span>` : ''}</div></div>
    ${d ? '<button class="btn small" id="csv">Download trades (CSV)</button>' : ''}</div>`;
}

// Plain-language summary for an analyst. Facts only, all from the stats.
function verdict(c, s) {
  if (!s.trades) return `The rules never triggered on ${esc(label(S.ticker))} in this window.`;
  const parts = [];
  parts.push(`${fx(s.trades, 0)} closed trades in ${fx(s.years, 1)} years, in the market ${pct(s.exposure, 0)} of the time.`);
  parts.push(`It compounded <b class="${neg(s.cagr)}">${pct(s.cagr, 2, true)}</b> a year against <b>${pct(s.bh_cagr, 2, true)}</b> for buy &amp; hold, with a worst drawdown of <b class="neg">${pct(s.max_dd, 1)}</b> (buy &amp; hold ${pct(s.bh_max_dd, 1)}).`);
  if (s.trades < 30) parts.push(`With fewer than 30 trades the statistics are too thin to trust.`);
  else parts.push(Math.abs(s.t_stat) >= 2 ? `The average trade is statistically different from zero (t = ${fx(s.t_stat)}).` : `The average trade is not statistically different from zero (t = ${fx(s.t_stat)}).`);
  const bw = c.claim && parseFloat(String(c.claim.win).replace(/[^\d.]/g, ''));
  if (bw) { const gap = s.win_rate * 100 - bw; parts.push(`Win rate ${pct(s.win_rate, 1)} vs ${esc(c.claim.win)} in the book (${gap >= 0 ? '+' : ''}${gap.toFixed(1)} pts).`); }
  return parts.join(' ');
}

// Rules grouped the way a trader reads them: what must be true, what triggers entry,
// how to add, how to get out. Lines are tagged F| E| A| X| by the backend.
function rulesPanel(c) {
  const sec = { F: [], E: [], A: [], X: [] };
  for (const r of c.rules) { const [tag, ...rest] = r.split('|'); (sec[tag] || sec.E).push(rest.join('|')); }
  const isOrder = r => /^(Buy|Short|Sell|Cover|Next day|No other signal|Once|After|Stop)/.test(r);
  const col = (title, hint, items, cls) => items.length ? `<div class="rcol ${cls}"><h4>${title}</h4><p class="rhint">${hint}</p><ul>${items.map(r => `<li class="${isOrder(r) ? 'order' : ''}">${esc(r)}</li>`).join('')}</ul></div>` : '';
  return `<div class="box rules"><div class="box-h">Rules <small>as written in the course, Session ${c.session}</small></div>
    <div class="rgrid" style="--cols:${[sec.F, sec.E, sec.A, sec.X].filter(x => x.length).length}">
      ${col('Conditions', 'All must be true (trend, liquidity, volatility filters)', sec.F, 'rf')}
      ${col('Entry', 'When all of these happen on a bar\'s close', sec.E, 're')}
      ${col('Scale-in', 'While the trade is open', sec.A, 'ra')}
      ${col('Exit', sec.X.length > 1 ? 'Whichever happens first' : 'How the trade is closed', sec.X, 'rx')}
    </div>
    ${c.notes ? `<p class="note">${esc(c.notes)}</p>` : ''}</div>`;
}

function metric(label, value, sub, cls = '') {
  return `<div class="metric"><span>${label}</span><b class="${cls}">${value}</b><i title="${sub.replace(/<[^>]+>/g, '')}">${sub}</i></div>`;
}

function renderSheet(c, d) {
  const s = d.stats, t = label(S.ticker), cl = c.claim;
  const openTrade = d.trades.find(x => x.open);
  const more = [
    ['Total return', pct(s.total_return, 1, true)], ['Buy & hold total return', pct(s.bh_total, 1, true)],
    ['Annual volatility', pct(s.vol, 1)], ['<abbr title="Mean daily return / downside deviation (target 0), annualised with √252">Sortino</abbr>', fx(s.sortino)],
    ['<abbr title="CAGR ÷ |max drawdown| over the whole test. (Calmar uses only the last 36 months.)">MAR ratio</abbr>', fx(s.mar)], ['Longest time under water', fx(s.max_dd_days, 0) + ' bars'],
    ['<abbr title="CAGR ÷ share of time in the market: what the strategy earns per unit of time it has capital at risk">CAGR per time in market</abbr>', s.exposure ? pct(s.cagr / s.exposure, 1, true) : '–'], ['Average capital deployed', pct(s.avg_deployed, 0)],
    ['<abbr title="Average return per closed trade = win rate × avg win − loss rate × avg loss">Expectancy per trade</abbr>', pct(s.avg_trade, 3)], ['Median trade', pct(s.median_trade, 3)],
    ['Average win', pct(s.avg_win)], ['Average loss', pct(s.avg_loss)],
    ['<abbr title="Average win ÷ |average loss|">Payoff ratio</abbr>', fx(s.payoff)], ['Longest losing streak', fx(s.max_consec_losses, 0) + ' trades'],
    ['Best trade', pct(s.best, 1, true)], ['Worst trade', pct(s.worst, 1)],
    ['Average bars held', fx(s.avg_bars, 1)], ['Trades per year', fx(s.trades_per_year, 1)],
  ];
  disposeCharts();
  $('#sheet').innerHTML = `
    <div class="box">
      ${titleBlock(c, d)}
      ${variantPanel(c)}
      <div class="metrics">
        ${metric('CAGR', pct(s.cagr, 2, true), `Buy &amp; hold ${pct(s.bh_cagr, 2, true)}`, neg(s.cagr))}
        ${metric('Sharpe (rf = 0)', fx(s.sharpe), `Sortino ${fx(s.sortino)}`, neg(s.sharpe))}
        ${metric('Max drawdown', pct(s.max_dd, 1), `Buy &amp; hold ${pct(s.bh_max_dd, 1)}`, 'neg')}
        ${metric('Win rate', pct(s.win_rate, 1), cl ? `Book ${esc(cl.win)}` : 'No book figure')}
        ${metric('Profit factor', fx(s.profit_factor), `Payoff ${fx(s.payoff)}`)}
        ${metric('Closed trades', fx(s.trades, 0), `t-stat ${fx(s.t_stat)}`)}
      </div>
      <p class="verdict">${verdict(c, s)}${openTrade ? ` One trade is still open (${pct(openTrade.ret, 2, true)} so far, not counted in the trade statistics).` : ''}</p>
    </div>

    ${rulesPanel(c)}

    <div class="box">
      <div class="box-h">Equity growth of ₹1
        <span class="chart-legend"><span><span class="swatch" style="background:${C.blue}"></span>Strategy</span>
        <label><input type="checkbox" id="showbh"> <span class="swatch" style="background:${C.orange}"></span>Compare with buy &amp; hold</label>
        <label><input type="checkbox" id="logscale" checked> Log scale</label></span></div>
      <div class="readout" id="eq-read"></div>
      <div class="lwc" id="c-eq" style="height:320px"></div>
      <div class="box-h" style="margin-top:16px">Drawdown <small>% below the previous equity peak</small></div>
      <div class="lwc" id="c-dd" style="height:130px"></div>
    </div>

    <div class="box">
      <div class="box-h">Price and trades
        <span class="chart-legend"><span><span class="dot" style="background:${C.blue}"></span>Entry / scale-in</span><span><span class="dot" style="background:${C.orange}"></span>Exit</span><button type="button" class="btn small" id="last3">Last 3 years</button><button type="button" class="btn small" id="fit">Show all</button></span></div>
      <div class="readout" id="px-read">Scroll to zoom and drag to pan. Click a trade in the log to jump to it.</div>
      <div class="lwc" id="c-px" style="height:340px"></div>
    </div>

    <div class="two">
      <div class="box"><div class="box-h">Book vs ${esc(t)} <small>${cl ? esc(cl.period) : 'The course gives no statistics'}</small></div>
        <table class="vs"><thead><tr><th></th><th class="n">Book</th><th class="n">${esc(t)}</th></tr></thead><tbody>
          <tr><td>Trades</td><td class="n">${cl ? esc(cl.trades) : '–'}</td><td class="n">${fx(s.trades, 0)}</td></tr>
          <tr><td>Win rate</td><td class="n">${cl ? esc(cl.win) : '–'}</td><td class="n">${pct(s.win_rate, 1)}</td></tr>
          <tr><td>Average trade</td><td class="n">${cl ? esc(cl.avg) : '–'}</td><td class="n ${neg(s.avg_trade)}">${pct(s.avg_trade)}</td></tr>
          <tr><td>Average hold</td><td class="n">${cl ? esc(cl.hold) : '–'}</td><td class="n">${fx(s.avg_bars, 1)} bars</td></tr>
        </tbody></table>
        <p class="note">Returns are on the full position size; scale-in pieces that never filled count as idle cash. Orders fill at the close unless a rule says next open or limit. Idle cash earns nothing and Sharpe uses a 0% risk-free rate, so both are conservative for India.</p></div>
    </div>

    <div class="box"><div class="box-h">All statistics <small>trade statistics use closed trades only</small></div><dl class="more">${more.map(([k, v]) => `<div><dt>${k}</dt><dd class="${String(v).startsWith('-') ? 'neg' : ''}">${v}</dd></div>`).join('')}</dl></div>

    <div class="two">
      <div class="box"><div class="box-h">Trade returns <small>closed trades per return bucket</small></div><div class="svgchart" id="c-hist"></div></div>
      <div class="box"><div class="box-h">Calendar-year returns <span class="chart-legend"><span><span class="swatch" style="background:${C.blue}"></span>Strategy</span><span><span class="swatch" style="background:${C.orange}"></span>Buy &amp; hold</span></span></div><div class="svgchart" id="c-yr"></div></div>
    </div>

    <div class="box"><div class="box-h">Monthly returns <small>strategy, % per month · blank = before the test window · "·" = no position</small></div><div class="scroll" id="heat"></div></div>
    <div class="box"><div class="box-h">Trade log <small>${d.trades.length} trades, newest first · indicator values are from the entry and exit bars · click a row to see it on the chart</small></div><div class="scroll" id="log"></div></div>`;

  drawEquity(d);
  drawPrice(d);
  histogram($('#c-hist'), d.trades.filter(x => !x.open).map(x => x.ret));
  annual($('#c-yr'), d);
  heatmap($('#heat'), d);
  tradeLog($('#log'), d);
  $('#csv').onclick = () => downloadCsv(c, d);
  wireVariants();
}

// ------------------------------------------------------------------ lightweight charts

window.addEventListener('resize', () => S.charts.forEach(ch => ch.applyOptions({ width: ch._el.clientWidth })));
function disposeCharts() { S.charts.forEach(ch => ch.remove()); S.charts = []; }

function baseChart(el, opts = {}) {
  const ch = LWC.createChart(el, {
    width: el.clientWidth,
    height: el.clientHeight,
    layout: { attributionLogo: false, background: { color: '#f4f4f4' }, textColor: C.ink3, fontFamily: getComputedStyle(document.body).fontFamily, fontSize: 11 },
    grid: { vertLines: { visible: false }, horzLines: { color: C.rule2 } },
    rightPriceScale: { borderColor: C.rule },
    timeScale: { borderColor: C.rule, minBarSpacing: 0.01 },
    crosshair: { mode: LWC.CrosshairMode.Normal, vertLine: { color: C.ink3, labelBackgroundColor: C.ink }, horzLine: { color: C.ink3, labelBackgroundColor: C.ink } },
    handleScale: { axisPressedMouseMove: false },
    ...opts,
  });
  ch._el = el;
  S.charts.push(ch);
  return ch;
}

function drawEquity(d) {
  const eq = baseChart($('#c-eq'), { rightPriceScale: { borderColor: C.rule, mode: LWC.PriceScaleMode.Logarithmic } });
  const fmt = { type: 'custom', formatter: v => v.toFixed(2) + '×', minMove: 0.01 };
  const bh = eq.addLineSeries({ color: C.orange, lineWidth: 2, priceFormat: fmt, lastValueVisible: false, priceLineVisible: false, visible: false });
  const st = eq.addLineSeries({ color: C.blue, lineWidth: 2, priceFormat: fmt, priceLineVisible: false });
  bh.setData(d.dates.map((t, i) => ({ time: t, value: d.bh[i] })));
  st.setData(d.dates.map((t, i) => ({ time: t, value: d.equity[i] })));

  const ddc = baseChart($('#c-dd'), { timeScale: { visible: false, borderColor: C.rule, minBarSpacing: 0.01 } });
  let pk = -Infinity;
  const dd = d.equity.map((v, i) => { pk = Math.max(pk, v); return { time: d.dates[i], value: (v / pk - 1) * 100 }; });
  ddc.addAreaSeries({ lineColor: C.red, topColor: 'rgba(196,60,60,0.04)', bottomColor: 'rgba(196,60,60,0.26)', lineWidth: 1.5, invertFilledArea: true, priceFormat: { type: 'custom', formatter: v => v.toFixed(1) + '%' }, priceLineVisible: false })
    .setData(dd);

  let lock = false; // keep both panes on the same window
  const sync = (a, b) => a.timeScale().subscribeVisibleLogicalRangeChange(r => { if (lock || !r) return; lock = true; b.timeScale().setVisibleLogicalRange(r); lock = false; });
  sync(eq, ddc); sync(ddc, eq);
  eq.timeScale().fitContent(); ddc.timeScale().fitContent();

  const read = $('#eq-read'), idx = Object.fromEntries(d.dates.map((t, i) => [t, i]));
  const show = i => { read.innerHTML = `<b>${d.dates[i]}</b>&emsp;Strategy ${d.equity[i].toFixed(2)}×&emsp;Buy &amp; hold ${d.bh[i].toFixed(2)}×&emsp;Drawdown <span class="neg">${dd[i].value.toFixed(1)}%</span>`; };
  show(d.dates.length - 1);
  eq.subscribeCrosshairMove(p => show(p.time ? idx[p.time] : d.dates.length - 1));
  $('#logscale').onchange = e => eq.priceScale('right').applyOptions({ mode: e.target.checked ? LWC.PriceScaleMode.Logarithmic : LWC.PriceScaleMode.Normal });
  $('#showbh').onchange = e => bh.applyOptions({ visible: e.target.checked });
}

function drawPrice(d) {
  const ch = baseChart($('#c-px'));
  const s = ch.addLineSeries({ color: C.ink, lineWidth: 1.5, priceLineVisible: false });
  s.setData(d.dates.map((t, i) => ({ time: t, value: d.close[i] })));
  const marks = [];
  for (const t of d.trades) {
    const long = t.dir > 0;
    for (const [i] of t.legs) marks.push({ time: d.dates[i], position: long ? 'belowBar' : 'aboveBar', color: C.blue, shape: long ? 'arrowUp' : 'arrowDown', size: 1 });
    if (!t.open) marks.push({ time: d.dates[t.exit_i], position: long ? 'aboveBar' : 'belowBar', color: C.orange, shape: 'circle', size: 0.7 });
  }
  marks.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0));
  s.setMarkers(marks);
  S.priceChart = ch;
  const n = d.dates.length, last3 = () => ch.timeScale().setVisibleLogicalRange({ from: Math.max(0, n - 756), to: n - 1 });
  last3(); // three years by default so individual trades are readable
  const idx = Object.fromEntries(d.dates.map((t, i) => [t, i])), read = $('#px-read');
  ch.subscribeCrosshairMove(p => {
    if (!p.time) return;
    const i = idx[p.time], tr = d.trades.find(t => i >= t.legs[0][0] && i <= t.exit_i);
    read.innerHTML = `<b>${d.dates[i]}</b>&emsp;Close ${d.close[i].toLocaleString('en-IN')}` + (tr ? `&emsp;In trade ${tr.entry} to ${tr.exit}: <b class="${neg(tr.ret)}">${pct(tr.ret, 2, true)}</b>${tr.legs.length > 1 ? ` (${tr.legs.length} pieces)` : ''}${tr.open ? ' · still open' : ''}` : '&emsp;<span class="muted">No position</span>');
  });
  $('#fit').onclick = () => ch.timeScale().fitContent();
  $('#last3').onclick = last3;
}

function focusTrade(t) {
  const d = S.detail, a = Math.max(0, t.legs[0][0] - 30), b = Math.min(d.dates.length - 1, t.exit_i + 30);
  S.priceChart.timeScale().setVisibleRange({ from: d.dates[a], to: d.dates[b] });
  $('#c-px').scrollIntoView({ behavior: 'smooth', block: 'center' });
}

// ------------------------------------------------------------------ small SVG charts

const NS = 'http://www.w3.org/2000/svg';
function niceTicks(lo, hi, n = 4) {
  const span = hi - lo || 1, step0 = span / n, mag = 10 ** Math.floor(Math.log10(step0));
  const step = [1, 2, 2.5, 5, 10].map(m => m * mag).find(s => s >= step0);
  const out = []; for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-12; v += step) out.push(+v.toFixed(12)); return out;
}
function tipEl(host) { let t = host.querySelector('.tip'); if (!t) { t = document.createElement('div'); t.className = 'tip'; host.appendChild(t); } return t; }

function bars(host, labels, groups, o) {
  const W = host.clientWidth || 460, H = 240, L = 46, R = 6, T = 8, B = 26;
  let lo = 0, hi = 0; for (const g of groups) for (const v of g.v) { lo = Math.min(lo, v); hi = Math.max(hi, v); }
  const Y = v => T + (H - T - B) * (1 - (v - lo) / ((hi - lo) || 1));
  const bw = (W - L - R) / labels.length, gap = 2, inner = Math.max(1, (bw - 3) / groups.length - gap);
  let g = '';
  for (const v of niceTicks(lo, hi)) g += `<line x1="${L}" x2="${W - R}" y1="${Y(v)}" y2="${Y(v)}" stroke="${C.rule2}"/><text x="${L - 6}" y="${Y(v) + 4}" text-anchor="end">${o.fmtY(v)}</text>`;
  const every = Math.ceil(labels.length / (W / 46));
  labels.forEach((lab, k) => {
    groups.forEach((gr, j) => {
      const v = gr.v[k], x = L + k * bw + 1.5 + j * (inner + gap), y0 = Y(0), y1 = Y(v);
      g += `<rect x="${x}" y="${Math.min(y0, y1)}" width="${inner}" height="${Math.max(1, Math.abs(y1 - y0))}" rx="${Math.min(3, inner / 3)}" fill="${gr.color(v, k)}"/>`;
    });
    if (k % every === 0) g += `<text x="${L + k * bw + bw / 2}" y="${H - 8}" text-anchor="middle">${lab}</text>`;
  });
  g += `<line x1="${L}" x2="${W - R}" y1="${Y(0)}" y2="${Y(0)}" stroke="${C.ink3}"/>`;
  const svg = document.createElementNS(NS, 'svg');
  svg.setAttribute('viewBox', `0 0 ${W} ${H}`); svg.setAttribute('width', '100%'); svg.innerHTML = g;
  host.innerHTML = ''; host.appendChild(svg);
  const tp = tipEl(host);
  svg.onmousemove = e => {
    const r = svg.getBoundingClientRect(), k = Math.floor(((e.clientX - r.left) * W / r.width - L) / bw);
    if (k < 0 || k >= labels.length) { tp.style.display = 'none'; return; }
    tp.innerHTML = o.tip(k); tp.style.display = 'block';
    const left = (L + k * bw + bw / 2) / W * r.width;
    tp.style.left = (left > r.width / 2 ? left - tp.offsetWidth - 10 : left + 10) + 'px'; tp.style.top = '6px';
  };
  svg.onmouseleave = () => tp.style.display = 'none';
}

function histogram(host, rets) {
  if (!rets.length) { host.innerHTML = '<div class="empty">No closed trades.</div>'; return; }
  const lo = Math.min(...rets), hi = Math.max(...rets), nb = Math.min(36, Math.max(8, Math.ceil(Math.sqrt(rets.length) * 1.5)));
  const w = (hi - lo) / nb || 0.001, counts = Array(nb).fill(0);
  for (const r of rets) counts[Math.min(nb - 1, Math.floor((r - lo) / w))]++;
  const mid = k => lo + (k + 0.5) * w;
  bars(host, counts.map((_, k) => pct(mid(k), 1)), [{ v: counts, color: (v, k) => (mid(k) >= 0 ? C.blue : C.red) }], {
    fmtY: v => v.toFixed(0), tip: k => `${pct(lo + k * w)} to ${pct(lo + (k + 1) * w)}<br><b>${counts[k]}</b> trades`,
  });
}

function periodReturns(dates, eq, len) {
  const out = {}; let base = eq[0], key = dates[0].slice(0, len);
  for (let i = 1; i < dates.length; i++) {
    const k = dates[i].slice(0, len);
    if (k !== key) { out[key] = eq[i - 1] / base - 1; base = eq[i - 1]; key = k; }
  }
  out[key] = eq[eq.length - 1] / base - 1; return out;
}

function annual(host, d) {
  const s = periodReturns(d.dates, d.equity, 4), b = periodReturns(d.dates, d.bh, 4), ys = Object.keys(s);
  bars(host, ys.map(y => "'" + y.slice(2)), [{ v: ys.map(y => s[y]), color: () => C.blue }, { v: ys.map(y => b[y]), color: () => C.orange }], {
    fmtY: v => pct(v, 0), tip: k => `<b>${ys[k]}</b><br>Strategy ${pct(s[ys[k]], 1, true)}<br>Buy &amp; hold ${pct(b[ys[k]], 1, true)}`,
  });
}

function heatmap(host, d) {
  const m = periodReturns(d.dates, d.equity, 7), ys = periodReturns(d.dates, d.equity, 4);
  const cap = Math.max(0.01, ...Object.values(m).map(Math.abs));
  const bg = v => {
    if (v == null) return 'transparent';
    if (Math.abs(v) < 1e-9) return 'var(--mid)';
    const a = Math.round((Math.min(1, Math.abs(v) / cap) * 0.8 + 0.12) * 100);
    return `color-mix(in srgb, ${v > 0 ? C.blue : C.red} ${a}%, #f4f4f4)`;
  };
  const M = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  host.innerHTML = `<table class="heat"><thead><tr><th>Year</th>${M.map(x => `<th>${x}</th>`).join('')}<th class="n">Year</th></tr></thead><tbody>` +
    Object.keys(ys).reverse().map(y => `<tr><th>${y}</th>${M.map((_, k) => {
      const v = m[`${y}-${String(k + 1).padStart(2, '0')}`];
      const strong = v != null && Math.abs(v) / cap > 0.5;
      return `<td style="background:${bg(v)};${strong ? 'color:#fff' : ''}" title="${M[k]} ${y}: ${v == null ? 'outside the test window' : pct(v, 2, true)}">${v == null ? '' : Math.abs(v) < 1e-9 ? '·' : (v * 100).toFixed(1)}</td>`;
    }).join('')}<td class="n ${neg(ys[y])}"><b>${pct(ys[y], 1, true)}</b></td></tr>`).join('') + '</tbody></table>';
}

// Re-check an exit against the values recorded on the bar where its rule fired.
function exitCheck(t) {
  const x = t.at_exit, r = t.reason, f1 = n => n == null ? '–' : n.toFixed(1), sg = n => (n > 0 ? '+' : '') + n.toFixed(2) + '%';
  let m = r.match(/^RSI\(2\) ([<>]) (\d+)/);
  if (m) { const th = +m[2]; return { ok: m[1] === '>' ? x.rsi2 > th : x.rsi2 < th, text: `RSI(2) was ${f1(x.rsi2)} ${m[1]} ${th}` }; }
  if (r === '5 bars held') return { ok: t.bars === 5, text: `held ${t.bars} bars, rule says 5` };
  if (r.startsWith('Close > 5-day SMA')) return { ok: x.vs_sma5 > 0, text: `close ${sg(x.vs_sma5)} vs 5-day SMA` };
  if (r.startsWith('Close < 5-day SMA')) return { ok: x.vs_sma5 < 0, text: `close ${sg(x.vs_sma5)} vs 5-day SMA` };
  if (r.includes('below 200-day SMA')) return { ok: x.vs_sma200 <= -1.5, text: `close ${sg(x.vs_sma200)} vs 200-day SMA` };
  if (r.includes('above 200-day SMA')) return { ok: x.vs_sma200 >= 1.5, text: `close ${sg(x.vs_sma200)} vs 200-day SMA` };
  if (r.startsWith('Close ≥ 5% profit')) { const g = x.close / t.avg_px - 1; return { ok: g >= 0.05, text: `close ${sg(g * 100)} vs average entry` }; }
  return null;
}

function tradeLog(host, d) {
  if (!d.trades.length) { host.innerHTML = '<div class="empty">The rules never triggered on this index in this window.</div>'; return; }
  const v = (x, dgt = 1) => x == null ? '–' : x.toFixed(dgt);
  const sgn = x => x == null ? '–' : (x > 0 ? '+' : '') + x.toFixed(1) + '%';
  const entryWhy = t => {
    const a = t.at_entry;
    return `${t.entry_kind === 'limit' ? '<span class="pill">limit fill</span> ' : ''}RSI(2) ${v(a.rsi2)} · ${sgn(a.vs_sma200)} vs 200-SMA` + (a.vix_rsi2 != null && a.vix_rsi2 !== undefined && S.catalogue.find(c => c.id === S.sel)?.needs_vix ? ` · VIX RSI(2) ${v(a.vix_rsi2)}` : '');
  };
  const exitWhy = t => {
    if (t.open) return '<span class="pill">still open, marked to last close</span>';
    const c = exitCheck(t);
    const when = t.signal !== t.exit ? `<br><span class="muted">rule fired on ${t.signal}'s close</span>` : '';
    return `<b>${esc(t.reason)}</b><br>${c ? `<span class="${c.ok ? 'ok' : 'bad'}">${c.ok ? '✓' : '✗'} ${c.text}</span>` : ''}${when}`;
  };
  const closed = d.trades.filter(t => !t.open), checked = closed.map(exitCheck).filter(Boolean), passed = checked.filter(c => c.ok).length;
  const summary = `<p class="checkline ${passed === checked.length ? 'ok' : 'bad'}">${passed === checked.length ? '✓' : '✗'} Exit rule re-checked against the bar values on ${checked.length} of ${closed.length} closed trades: <b>${passed} pass</b>${passed < checked.length ? `, <b>${checked.length - passed} fail</b>` : ''}.</p>`;
  host.innerHTML = summary + `<table class="log"><thead><tr><th class="n">#</th><th>Entry</th><th>Entry signal (bar values)</th><th>Exit</th><th>Exit reason</th><th class="n">Pieces</th><th class="n">Avg entry</th><th class="n">Exit price</th><th class="n">Bars</th><th class="n">Return</th></tr></thead><tbody>` +
    d.trades.map((t, k) => ({ t, k })).reverse().map(({ t, k }) => `<tr class="click" data-k="${k}"><td class="n muted">${k + 1}</td><td>${t.entry}</td><td class="why">${entryWhy(t)}</td><td>${t.open ? '–' : t.exit}</td><td class="why">${exitWhy(t)}</td><td class="n">${t.legs.length}</td><td class="n">${fx(t.avg_px)}</td><td class="n">${fx(t.exit_px)}</td><td class="n">${t.bars}</td><td class="n ${neg(t.ret)}"><b>${pct(t.ret, 2, true)}</b></td></tr>`).join('') + '</tbody></table>';
  host.querySelectorAll('tr.click').forEach(tr => tr.onclick = () => focusTrade(d.trades[+tr.dataset.k]));
}

function downloadCsv(c, d) {
  const rows = [['entry', 'exit', 'direction', 'entry_kind', 'rsi2_at_entry', 'pct_vs_sma200_at_entry', 'exit_reason', 'rsi2_at_exit', 'pieces', 'avg_entry', 'exit_price', 'bars', 'return', 'open']]
    .concat(d.trades.map(t => [t.entry, t.exit, t.dir > 0 ? 'long' : 'short', t.entry_kind, t.at_entry.rsi2, t.at_entry.vs_sma200, `"${t.reason}"`, t.at_exit.rsi2, t.legs.length, t.avg_px, t.exit_px, t.bars, t.ret, t.open]));
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([rows.map(r => r.join(',')).join('\n')], { type: 'text/csv' }));
  a.download = `connors_${c.id}_${S.ticker.replace(/\s/g, '')}.csv`;
  a.click();
}

// ------------------------------------------------------------------ other tabs

function renderCompare() {
  const winNum = s => { const m = String(s || '').match(/[\d.]+/); return m ? parseFloat(m[0]) / 100 : null; };
  const rows = S.catalogue.filter(c => c.status === 'live' && c.claim && winNum(c.claim.win) != null && S.summary[c.id]?.stats?.trades)
    .map(c => { const s = S.summary[c.id].stats, b = winNum(c.claim.win); return { c, s, b, i: s.win_rate, gap: s.win_rate - b }; });
  const by = $('#tab-compare').dataset.sort || 'gap';
  rows.sort((x, y) => by === 'gap' ? y.gap - x.gap : y.s.sharpe - x.s.sharpe);
  const X = v => `${Math.max(0, Math.min(100, (v - 0.3) / 0.7 * 100))}%`; // axis 30%..100%
  const held = rows.filter(r => r.gap >= -0.05).length;
  $('#tab-compare').innerHTML = `
    <div class="box">
      <div class="box-h">Does the book's win rate hold on ${esc(label(S.ticker))}?
        <span class="chart-legend"><span><span class="dot" style="background:${C.ink3}"></span>Book</span><span><span class="dot" style="background:${C.blue}"></span>India, within 5 pts</span><span><span class="dot" style="background:${C.red}"></span>India, more than 5 pts worse</span>
        <span class="seg"><button type="button" data-s="gap" class="${by === 'gap' ? 'on' : ''}">Sort by gap</button><button type="button" data-s="sharpe" class="${by === 'sharpe' ? 'on' : ''}">Sort by Sharpe</button></span></span></div>
      <p class="muted" style="margin:0 0 12px">${held} of ${rows.length} strategies keep their published win rate within 5 points. A high win rate says nothing about payoff, so read it with the Sharpe on the right. Rows with under 30 trades are statistically thin.</p>
      <div class="cmp-axis"><span></span><div class="ticks">${[0.3, 0.5, 0.7, 0.9, 1].map(v => `<span style="left:${X(v)}">${v * 100}%</span>`).join('')}</div><span style="text-align:right">Gap · Sharpe</span></div>
      ${rows.map(r => {
        const lo = Math.min(r.b, r.i), hi = Math.max(r.b, r.i);
        return `<div class="cmp-row" data-id="${r.c.id}"><div class="cmp-name">${esc(fullName(r.c))}<small>${fx(r.s.trades, 0)} trades on ${esc(label(S.ticker))}${r.s.trades < 30 ? ' (thin)' : ''} · book ${esc(r.c.claim.trades)}</small></div>
          <div class="track"><span class="bar" style="left:${X(lo)};width:calc(${X(hi)} - ${X(lo)})"></span><span class="pt book" style="left:${X(r.b)}" title="Book ${pct(r.b, 1)}"></span><span class="pt india${r.gap < -0.05 ? ' worse' : ''}" style="left:${X(r.i)}" title="${esc(label(S.ticker))} ${pct(r.i, 1)}"></span></div>
          <div class="cmp-delta"><span class="${neg(r.gap)}">${(r.gap > 0 ? '+' : '') + (r.gap * 100).toFixed(1)} pts</span> · <span class="${neg(r.s.sharpe)}">${fx(r.s.sharpe)}</span></div></div>`;
      }).join('')}
    </div>`;
  $('#tab-compare').querySelectorAll('.seg button').forEach(b => b.onclick = () => { $('#tab-compare').dataset.sort = b.dataset.s; renderCompare(); });
  $('#tab-compare').querySelectorAll('.cmp-row').forEach(el => el.onclick = () => openStrategy(el.dataset.id, true));
}

function renderThesis() {
  $('#tab-thesis').innerHTML = `<p class="thesis-lede">${esc(S.guru.tagline)}</p>
    <div class="thesis">${S.guru.thesis.map(t => `<article><h3>${esc(t.title)}</h3><p>${esc(t.body)}</p></article>`).join('')}</div>`;
}

function renderBlocked() {
  const rows = S.catalogue.filter(c => c.status !== 'live');
  $('#tab-blocked').innerHTML = `<div class="box"><div class="box-h">In the course, but the data to test them is missing <small>listed as taught, never approximated</small></div>
    <table><thead><tr><th>Strategy</th><th>Missing data</th><th>Rules</th><th class="n">Book win / avg</th></tr></thead><tbody>` +
    rows.map(c => `<tr><td><b>${esc(c.name)}</b><br><span class="muted">${esc(c.family_name)}</span></td><td><span class="pill">${esc(c.status_note)}</span></td><td style="white-space:normal;max-width:60ch">${esc(c.notes)}</td><td class="n muted">${c.claim ? esc(c.claim.win) + ' / ' + esc(c.claim.avg) : '–'}</td></tr>`).join('') +
    '</tbody></table></div>';
}

init().catch(e => { $('#status').textContent = `Could not reach the Rust API at ${API} (${e.message})`; });
