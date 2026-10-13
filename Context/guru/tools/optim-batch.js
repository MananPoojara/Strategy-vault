// Batch optimization of every live strategy on every index, with the same defaults as the strategy page
// (grid = every parameter over its full range, steps widened until ≤ 50,000 settings; split at 60% of the window; cost 0).
// Writes ../frontend/data/optim-batch.json, the evidence behind data/claude-conclusions.json.
//   node optim-batch.js [india api] [us api]
const path = require('path'), fs = require('fs');
const OC = require('../frontend/optim-core.js');
// --book <id> runs only that book's strategies and merges them into the existing file.
const bi = process.argv.indexOf('--book'), BOOK = bi > 0 ? process.argv[bi + 1] : null;
const IN = 'http://localhost:8090', US = 'http://localhost:8091';
// US test window per book (the book's own test period) and the first day after publication
const US_WIN = { stc9: ['1995-01-01', '2009-12-31', '2010-01-01'], etfss: ['2000-01-03', '2004-06-16', '2004-06-17'] };
const get = async u => { const r = await fetch(u), t = await r.text(); if (!r.ok) throw new Error(t); return JSON.parse(t); };

(async () => {
  const [cat, schema, tin, tus] = await Promise.all([get(`${IN}/api/guru/connors/strategies`), get(`${IN}/api/guru/connors/params`), get(`${IN}/api/tickers`), get(`${US}/api/tickers`)]);
  const usFor = v => (v.book || 'stc9') === 'stc9' ? tus.filter(t => t.ticker === 'SPY') : tus;
  const marketsFor = v => { const [f, t2] = US_WIN[v.book || 'stc9']; return [...tin.map(t => ({ api: IN, t, from: '', to: '' })), ...usFor(v).map(t => ({ api: US, t, from: f, to: t2 }))]; };
  const out = { generated: new Date().toISOString(), method: 'Every parameter searched over its full range (steps widened until the grid is at most 50,000 settings, always including the book value). In-sample = first ~60% of years, out-of-sample = the rest. A setting is judged only if it traded in both periods. Claude’s pick = highest plateau score (median in-sample Sharpe of the setting and its neighbours) within the strongest evidence tier: edge significant at 95% (Student t-test on trade returns, so small samples need a larger t) both in-sample and out-of-sample with all 3 out-of-sample checks passed, then significant in-sample with all checks passed, then all checks passed, then positive out-of-sample Sharpe. Deflated Sharpe uses N = grid size. Before costs.', strategies: {},
    data_notes: [
      'NIFTYIT data in the vault ends on 2025-06-02 (no newer source in the project); the other Indian indices run to 2026-09-29.',
      'India VIX daily bars are built from 1-minute data. Its first print each morning sits at about the previous close: it opens above the prior day\'s high on only 0.28% of days (8 of 2,896, 2015-2026) against 19.67% for the CBOE VIX (1990-2026). So "VIX gaps above yesterday\'s high" (vix7) is structurally unusable in India, and "VIX opens above yesterday\'s close" (vix2) is close to a coin flip there (51.5% of days, CBOE 53.8%).',
      'The book was published in 2010. For SPY, post.book and post.pick give 2010-2026 results for the book settings and for the pick chosen on 1995-2009 data: a true out-of-sample test of both.',
      'ETF Sweet Spot (book etfss) was published in August 2004 and tested on SPY, QQQ, IWM, SMH and MDY from 2000-01-03 to 2004-06-16. For those five US ETFs, post holds 2004-06-17 to 2026 results for the book settings and for the pick chosen on the book window. QQQ, IWM, SMH and MDY are Yahoo daily data, split-adjusted (percent returns unaffected). Our engine reproduced the book: SPY 2003 trade list 19 of 20 units identical (the missing add-on was an intraday signal), SPY shorts 52/52 units and 53.23 points exactly, IWM 58/58 units; details in book2/verify-report.json.',
    ] };
  for (const v of cat.filter(x => x.status === 'live' && (!BOOK || (x.book || 'stc9') === BOOK))) {
    const markets = marketsFor(v);
    const ps = schema[v.id], g = Object.fromEntries(Object.entries(OC.defaultGrid(ps, 50000)).map(([k, x]) => [k, `${x.min}:${x.max}:${x.step}`]));
    const res_ = out.strategies[v.id] = { name: `${v.family_name}${v.variant ? ' · ' + v.variant : ''}`, rules: v.rules, claim: v.claim, params: ps, indices: {} };
    for (const m of markets) {
      const tk = m.t.ticker;
      if (v.needs_volume && !m.t.has_volume) { res_.indices[tk] = { blocked: 'no volume data' }; continue; }
      try {
        const q = new URLSearchParams({ id: v.id, ticker: tk, from: m.from, to: m.to, cost: '0' });
        const book = await get(`${m.api}/api/guru/connors/run?${q}`);
        const y0 = +book.dates[0].slice(0, 4), y1 = +book.dates[book.dates.length - 1].slice(0, 4), split = `${y0 + Math.round((y1 - y0) * 0.6)}-01-01`;
        const oq = new URLSearchParams(q); oq.set('split', split); Object.entries(g).forEach(([k, s]) => oq.set('g.' + k, s));
        const res = await get(`${m.api}/api/guru/connors/optimize?${oq}`);
        res.defaults = Object.fromEntries(ps.map(p => [p.key, p.def]));
        const an = OC.analyze(res);
        if (an.robust) {
          const pq = new URLSearchParams(q); Object.entries(an.robust.p).forEach(([k, x]) => pq.set('p.' + k, x));
          const d = await get(`${m.api}/api/guru/connors/run?${pq}`); an.dsr = OC.dsrOfEquity(d.equity, an.sharpes, res.combos);
        }
        const c = OC.conclude(an, Object.fromEntries(ps.map(p => [p.key, p.label])));
        const pick = r => r && { p: r.p, is_book: r.isBook, plateau: r.plateau, tier: an.tiers[r.tier], checks: r.checks, ...Object.fromEntries(['sharpe', 'is_sharpe', 'oos_sharpe', 'cagr', 'max_dd', 'mar', 'trades', 'win_rate', 'profit_factor', 'avg_trade', 't_stat', 'is_t_stat', 'oos_t_stat', 'oos_trades', 'is_trades', 'oos_max_dd', 'is_max_dd', 'exposure'].map(k => [k, r.m[k]])) };
        // evidence for the "how we got here" panel: the settings funnel and the Sharpe distribution of the whole grid
        const funnel = { tested: res.combos, judged: an.gated.length, checks: an.gated.filter(r => r.passes === 3 || (!res.split && r.gated)).length, significant: an.gated.filter(r => r.tier === 0).length };
        const sh = an.rows.map(r => r.m.sharpe).filter(Number.isFinite), lo = Math.min(...sh), hi = Math.max(...sh), nb = 24, w = (hi - lo) / nb || 1, bins = Array(nb).fill(0);
        sh.forEach(x => bins[Math.min(nb - 1, Math.floor((x - lo) / w))]++);
        // US only: the book was published in 2010, so 2010 onwards is a true out-of-sample test of both the book and the pick
        let post = null;
        if (m.api === US && an.robust) {
          const pq = new URLSearchParams({ id: v.id, ticker: tk, from: US_WIN[v.book || 'stc9'][2], to: '', cost: '0' }), tq = new URLSearchParams(pq);
          Object.entries(an.robust.p).forEach(([k, x]) => tq.set('p.' + k, x));
          const keep = s => Object.fromEntries(['sharpe', 'cagr', 'max_dd', 'trades', 'win_rate', 't_stat', 'exposure'].map(k => [k, s[k]]));
          const [b2, t2] = await Promise.all([get(`${m.api}/api/guru/connors/run?${pq}`), get(`${m.api}/api/guru/connors/run?${tq}`)]);
          post = { from: b2.dates[0], to: b2.dates[b2.dates.length - 1], book: keep(b2.stats), pick: keep(t2.stats) };
        }
        res_.indices[tk] = { funnel, hist: { lo, w, bins }, post, market: m.api === US ? 'US (book data)' : 'India', from: res.from, to: res.to, split: res.split, combos: res.combos, ms: res.ms, grid: g, judged: an.gated.length, book: an.book, book_pct: an.bookPct,
          robust: pick(an.robust), skipped: an.skipped, dsr: an.dsr && an.dsr.dsr, sr0: an.dsr && an.dsr.sr0Ann, others: an.picks.slice(1).map(x => ({ kind: x.kind, ...pick(x.row) })), verdict: c.verdict, conviction: c.conviction, recommendation: c.recommendation, evidence: c.evidence, caveats: c.caveats };
        process.stdout.write(`${v.id} ${tk}: ${res.combos} combos ${res.ms}ms → ${c.verdict}\n`);
      } catch (e) { res_.indices[tk] = { error: e.message }; process.stdout.write(`${v.id} ${tk}: ${e.message}\n`); }
    }
  }
  const f = path.join(__dirname, '../frontend/data/optim-batch.json');
  if (BOOK && fs.existsSync(f)) { const old = JSON.parse(fs.readFileSync(f, 'utf8')); Object.assign(old.strategies, out.strategies); old.data_notes = [...new Set([...(old.data_notes || []), ...out.data_notes])]; old.generated = out.generated; fs.writeFileSync(f, JSON.stringify(old, null, 1)); }
  else fs.writeFileSync(f, JSON.stringify(out, null, 1));
  console.log('wrote', f);
})();
