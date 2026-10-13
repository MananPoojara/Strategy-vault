// Optimization analysis shared by the strategy page (browser) and the batch report (node).
// Input: the JSON of /api/guru/connors/optimize. Ranking rule (see verification/formula-verification.md):
//   gate   — enough trades in full, in-sample and out-of-sample periods
//   rank   — median in-sample Sharpe of the setting and its ±1-step grid neighbours (a plateau, not a spike)
//   check  — out-of-sample Sharpe > 0, OOS/IS Sharpe ≥ 0.5, OOS max DD ≤ 1.5 × IS max DD (shown, not ranked on)
//   badge  — Deflated Sharpe Ratio (Bailey & López de Prado 2014) with N = grid size
(function (root) {
  const median = a => { const s = [...a].sort((x, y) => x - y), n = s.length; return n ? (n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2) : NaN; };
  const num = v => (v == null || !isFinite(v) ? NaN : v);

  // Normal CDF (Abramowitz-Stegun 7.1.26) and its inverse (Acklam).
  function ncdf(x) { const t = 1 / (1 + 0.3275911 * Math.abs(x) / Math.SQRT2), y = 1 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * Math.exp(-x * x / 2); return x >= 0 ? (1 + y) / 2 : (1 - y) / 2; }
  function ninv(p) {
    const a = [-39.69683028665376, 220.9460984245205, -275.9285104469687, 138.357751867269, -30.66479806614716, 2.506628277459239], b = [-54.47609879822406, 161.5858368580409, -155.6989798598866, 66.80131188771972, -13.28068155288572],
      c = [-0.007784894002430293, -0.3223964580411365, -2.400758277161838, -2.549732539343734, 4.374664141464968, 2.938163982698783], d = [0.007784695709041462, 0.3224671290700398, 2.445134137142996, 3.754408661907416], pl = 0.02425;
    let q, r;
    if (p < pl) { q = Math.sqrt(-2 * Math.log(p)); return (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1); }
    if (p > 1 - pl) { q = Math.sqrt(-2 * Math.log(1 - p)); return -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1); }
    q = p - 0.5; r = q * q; return (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1);
  }
  const EMC = 0.5772156649;
  // Expected max Sharpe of N trials with Sharpe variance V (same frequency as SR) when the true Sharpe is 0.
  const expMaxSR = (V, N) => N < 2 ? 0 : Math.sqrt(V) * ((1 - EMC) * ninv(1 - 1 / N) + EMC * ninv(1 - 1 / (N * Math.E)));
  // sr: daily Sharpe of the chosen setting, T: number of daily returns, kurt: non-excess (normal = 3).
  const dsr = (sr, T, skew, kurt, V, N) => ncdf((sr - expMaxSR(V, N)) * Math.sqrt(T - 1) / Math.sqrt(Math.max(1e-12, 1 - skew * sr + (kurt - 1) / 4 * sr * sr)));
  /** DSR of one run's equity curve, given the annualised Sharpes of every grid setting. */
  function dsrOfEquity(equity, gridSharpesAnn, N) {
    const r = equity.length ? [equity[0] - 1] : []; for (let i = 1; i < equity.length; i++) r.push(equity[i] / equity[i - 1] - 1);
    const T = r.length, m = r.reduce((a, b) => a + b, 0) / T, sd = Math.sqrt(r.reduce((a, x) => a + (x - m) ** 2, 0) / (T - 1));
    if (!(sd > 0)) return null;
    const skew = r.reduce((a, x) => a + ((x - m) / sd) ** 3, 0) / T, kurt = r.reduce((a, x) => a + ((x - m) / sd) ** 4, 0) / T;
    const daily = gridSharpesAnn.filter(isFinite).map(s => s / Math.sqrt(252)), mu = daily.reduce((a, b) => a + b, 0) / daily.length;
    const V = daily.reduce((a, x) => a + (x - mu) ** 2, 0) / Math.max(1, daily.length - 1);
    return { dsr: dsr(m / sd, T, skew, kurt, V, N), sr0Ann: expMaxSR(V, N) * Math.sqrt(252), skew, kurt, T };
  }

  /** Month-on-month returns from a daily equity curve: month-end equity / previous month-end − 1. */
  function monthly(dates, equity) {
    const o = {}; let base = 1, key = dates[0].slice(0, 7);
    dates.forEach((t, i) => { const k = t.slice(0, 7); if (k !== key) { o[key] = equity[i - 1] / base - 1; base = equity[i - 1]; key = k; } });
    o[key] = equity[equity.length - 1] / base - 1; return o;
  }

  const r9 = x => Math.round(x * 1e9) / 1e9;
  const nVals = g => g.step > 0 && g.max >= g.min ? Math.floor((g.max - g.min) / g.step + 1e-9) + 1 : 0;
  /** Range for one parameter at a given step, aligned so the book value is always one of the grid points. */
  const aligned = (p, step) => ({ min: r9(p.def - Math.floor((p.def - p.min) / step + 1e-9) * step), max: r9(p.def + Math.floor((p.max - p.def) / step + 1e-9) * step), step: r9(step) });
  /** Default search: every parameter over its full range; the step of the widest dimension is doubled until the grid fits. */
  function defaultGrid(ps, limit = 50000) {
    const g = {}; ps.forEach(p => g[p.key] = { on: true, ...aligned(p, p.step) });
    const size = () => Object.values(g).reduce((a, x) => a * nVals(x), 1);
    for (let guard = 0; size() > limit && guard < 200; guard++) {
      const k = Object.keys(g).sort((a, b) => nVals(g[b]) - nVals(g[a]))[0];
      Object.assign(g[k], aligned(ps.find(x => x.key === k), g[k].step * 2));
    }
    return g;
  }

  // Ranking. No arbitrary trade minimum: a setting is judged only if it traded in BOTH halves (otherwise there is
  // nothing to test out of sample). Evidence is graded with a Student t-test on trade returns, which demands a
  // larger t from a small sample. Claude prefers, in order:
  //   0. edge significant in-sample AND out-of-sample, and all 3 out-of-sample checks pass
  //   1. significant in-sample, all 3 checks pass
  //   2. all 3 checks pass
  //   3. positive out-of-sample Sharpe
  //   4. anything testable
  // and within the best non-empty tier takes the highest plateau score (median in-sample Sharpe of the setting and its neighbours).
  // Two-sided 95% critical values of Student's t: a 3-trade sample needs t ≥ 4.30, 10 trades 2.26, 60+ about 2.
  const TCRIT = [NaN, 12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042];
  const tCrit = df => df < 1 ? Infinity : df <= 30 ? TCRIT[df] : df <= 40 ? 2.021 : df <= 60 ? 2.000 : df <= 120 ? 1.980 : 1.960;
  const significant = (t, n) => num(t) >= tCrit(n - 1);
  const TIERS = ['significant both in-sample and out-of-sample, and pass all three out-of-sample checks', 'significant in-sample and pass all three out-of-sample checks', 'pass all three out-of-sample checks', 'have a positive out-of-sample Sharpe', 'are testable'];
  function analyze(res) {
    const nd = res.dims.length, split = !!res.split, sizes = res.dims.map(d => d.values.length);
    const rows = res.rows.map((r, idx) => {
      const m = {}; res.cols.forEach((c, i) => { if (i >= nd) m[c] = r[i]; });
      const p = {}; res.dims.forEach((d, j) => p[d.key] = r[j]);
      return { idx, p, m };
    });
    const bookM = {}; res.cols.slice(nd).forEach((c, i) => bookM[c] = res.book[i]);
    const rankSharpe = m => num(split ? m.is_sharpe : m.sharpe);
    const pos = idx => { const out = Array(nd); for (let j = nd - 1; j >= 0; j--) { out[j] = idx % sizes[j]; idx = Math.floor(idx / sizes[j]); } return out; };
    const idxOf = ps => ps.reduce((a, v, j) => a * sizes[j] + v, 0);
    const testable = m => split ? m.is_trades > 0 && m.oos_trades > 0 : m.trades > 0;
    for (const row of rows) {
      const ps = pos(row.idx), around = [rankSharpe(row.m)];
      for (let j = 0; j < nd; j++) for (const dlt of [-1, 1]) { const q = ps.slice(); q[j] += dlt; if (q[j] >= 0 && q[j] < sizes[j]) around.push(rankSharpe(rows[idxOf(q)].m)); }
      row.plateau = median(around.map(v => isFinite(v) ? v : 0));
      row.neighbours = around.length - 1;
      const m = row.m;
      row.gated = testable(m);
      row.checks = split ? {
        oos_positive: num(m.oos_sharpe) > 0,
        oos_vs_is: num(m.is_sharpe) > 0 ? num(m.oos_sharpe) / m.is_sharpe >= 0.5 : num(m.oos_sharpe) > 0,
        oos_dd: Math.abs(num(m.oos_max_dd)) <= 1.5 * Math.abs(num(m.is_max_dd)) + 1e-12,
      } : null;
      row.passes = row.checks ? Object.values(row.checks).filter(Boolean).length : null;
      row.sig = split ? significant(m.is_t_stat, m.is_trades) : significant(m.t_stat, m.trades);
      row.sigOos = split && significant(m.oos_t_stat, m.oos_trades);
      row.tier = !row.gated ? 9 : !split ? (row.sig ? 0 : 4) : row.passes === 3 ? (row.sig ? (row.sigOos ? 0 : 1) : 2) : row.checks.oos_positive ? 3 : 4;
      row.isBook = res.dims.every(d => Math.abs(row.p[d.key] - (res.defaults?.[d.key] ?? NaN)) < 1e-9);
    }
    const gated = rows.filter(r => r.gated), byScore = [...gated].sort((a, b) => b.plateau - a.plateau || num(b.m.sharpe) - num(a.m.sharpe));
    const best = (list, f) => list.reduce((a, r) => (isFinite(f(r)) && (!a || f(r) > f(a)) ? r : a), null);
    const topTier = gated.length ? Math.min(...gated.map(r => r.tier)) : null;
    const robust = byScore.find(r => r.tier === topTier) || null;
    const skipped = robust ? byScore.indexOf(robust) : 0;
    const picks = [];
    if (robust) picks.push({ kind: 'robust', title: 'Claude’s pick', why: `Highest plateau score (median ${split ? 'in-sample ' : ''}Sharpe of this setting and its ${robust.neighbours} neighbours) among settings that are ${TIERS[robust.tier]}.${skipped ? ` ${skipped} setting${skipped > 1 ? 's' : ''} with a higher score ${skipped > 1 ? 'were' : 'was'} passed over for weaker evidence.` : ''}`, row: robust });
    if (split) { const o = best(gated, r => num(r.m.oos_sharpe)); if (o && o !== robust) picks.push({ kind: 'oos', title: 'Best out-of-sample', why: 'Highest Sharpe after the split date. Chosen by looking at the hold-out, so that number is not a clean test.', row: o }); }
    const mar = best(gated, r => num(r.m.mar)); if (mar && !picks.some(p => p.row === mar)) picks.push({ kind: 'mar', title: 'Best return / drawdown', why: 'Highest MAR (CAGR ÷ |max drawdown|) over the full period.', row: mar });
    const bookSharpe = num(bookM.sharpe), withS = gated.filter(r => isFinite(num(r.m.sharpe)));
    const bookPct = withS.length && isFinite(bookSharpe) ? withS.filter(r => r.m.sharpe <= bookSharpe).length / withS.length : null;
    return { skipped, res, rows, gated, top: byScore.slice(0, 25), picks, robust, book: bookM, bookPct, split, tiers: TIERS, sharpes: rows.map(r => num(r.m.sharpe)) };
  }

  const f2 = v => (v == null || !isFinite(v) ? '–' : v.toFixed(2));
  const p2 = v => (v == null || !isFinite(v) ? '–' : (v > 0 ? '+' : '') + (v * 100).toFixed(2) + '%');
  const CHECK_TXT = { oos_positive: 'positive out-of-sample Sharpe', oos_vs_is: 'out-of-sample Sharpe at least half of in-sample', oos_dd: 'out-of-sample drawdown within 1.5× in-sample' };
  /** Analyst reading of one optimization run: verdict, conviction, recommendation, evidence, caveats. */
  function conclude(an, labels = {}) {
    const L = k => labels[k] || k, set = p => Object.entries(p).map(([k, v]) => `${L(k)} ${v}`).join(', ');
    const { res, robust, book } = an, ev = [], cav = [];
    const stat = (m, t) => `Sharpe ${f2(m.sharpe)}${an.split ? ` (in-sample ${f2(m.is_sharpe)}, out-of-sample ${f2(m.oos_sharpe)})` : ''}, CAGR ${p2(m.cagr)}, max drawdown ${p2(m.max_dd)}, ${m.trades ?? 0} trades, t-stat ${f2(t ?? m.t_stat)}`;
    ev.push(`Search: ${res.combos.toLocaleString('en-IN')} settings over ${res.from} to ${res.to}${an.split ? `; in-sample to ${res.split}, out-of-sample after` : ''}. ${an.gated.length.toLocaleString('en-IN')} traded in both periods and could be judged.`);
    ev.push(`Book settings: ${stat(book)}.${an.bookPct != null ? ` Better than ${(an.bookPct * 100).toFixed(0)}% of the judged settings.` : ''}`);
    if (!robust) return { verdict: 'No testable setting', conviction: 'None', recommendation: 'No setting traded on both sides of the split. Move the split date or widen the period before drawing any conclusion.', evidence: ev, caveats: cav, text: ev };
    const m = robust.m, gain = num(m.sharpe) - num(book.sharpe), d = an.dsr?.dsr;
    if (!robust.isBook) ev.push(`Best robust setting: ${set(robust.p)}. ${stat(m)}. Plateau score ${f2(robust.plateau)}.`);
    if (an.split) ev.push(`Out-of-sample checks: ${robust.passes}/3 passed${robust.passes < 3 ? ` (failed: ${Object.entries(robust.checks).filter(([, ok]) => !ok).map(([k]) => CHECK_TXT[k]).join('; ')})` : ''}.`);
    if (d != null) ev.push(`Luck test: Deflated Sharpe ${d.toFixed(2)} after ${res.combos.toLocaleString('en-IN')} trials; the best of that many random settings would show a Sharpe near ${f2(an.dsr.sr0Ann)}.`);
    let verdict, conviction, rec;
    if (an.split && !robust.checks.oos_positive) { verdict = 'Avoid'; conviction = 'High'; rec = 'Do not trade this rule on this index and period: even the most robust setting loses money out of sample.'; }
    else if (robust.isBook || gain < 0.1) { verdict = 'Keep the book settings'; conviction = robust.sig && (d == null || d >= 0.95) ? 'High' : 'Medium'; rec = `Stay with the book settings. The search could not beat them by a meaningful margin (Sharpe gain ${f2(gain)}), so any change would mostly fit noise.`; }
    else {
      const strong = robust.tier === 0 && d != null && d >= 0.95;
      verdict = strong ? 'Use the tuned settings' : robust.passes === 3 || !an.split ? 'Tuned settings: promising, not proven' : 'Tune with caution';
      conviction = strong ? 'High' : robust.tier <= 1 && (d == null || d >= 0.8) ? 'Medium' : 'Low';
      rec = strong ? `Switch to ${set(robust.p)}. It adds ${f2(gain)} Sharpe, is statistically significant in both periods, passes the out-of-sample checks and clears the luck test.`
        : `Paper-trade ${set(robust.p)} alongside the book version before switching. It adds ${f2(gain)} Sharpe, but ${robust.tier > 0 ? 'its edge is not statistically significant in both periods' : 'it does not clear the luck test'}.`;
    }
    if (an.split ? m.oos_trades < 10 : m.trades < 20) cav.push(`Small sample: the pick has ${m.trades} trades${an.split ? ` (${m.oos_trades} out of sample)` : ''}, so a handful of trades can swing the result.`);
    if (num(m.max_dd) < num(book.max_dd) - 0.05) cav.push(`Deeper drawdown than the book (${p2(m.max_dd)} against ${p2(book.max_dd)}).`);
    if (an.split && an.skipped) cav.push(`The out-of-sample result was used to choose between settings, so the hold-out is no longer a fully independent test.`);
    if (d != null && d < 0.95 && !robust.isBook) cav.push(`Deflated Sharpe below 0.95: the improvement may come from searching many settings rather than a real edge.`);
    return { verdict, conviction, recommendation: rec, evidence: ev, caveats: cav, text: [rec, ...ev, ...cav] };
  }

  const api = { tCrit, analyze, conclude, monthly, dsr, expMaxSR, dsrOfEquity, median, defaultGrid, nVals };
  if (typeof module !== 'undefined' && module.exports) module.exports = api; else root.OptimCore = api;

  // self-check: node optim-core.js
  if (typeof require !== 'undefined' && typeof module !== 'undefined' && require.main === module) {
    const assert = require('assert'), f = n => dsr(2.5 / Math.sqrt(250), 1250, -3, 10, 0.5 / 250, n);
    assert(Math.abs(f(100) - 0.9004) < 1e-3 && Math.abs(f(46) - 0.9505) < 1e-3, 'DSR paper example');
    // 3×3 grid: the centre is a plateau, a corner is a spike
    const sh = [0.1, 0.1, 2.0, 0.8, 0.9, 0.1, 0.4, 0.8, 0.4];
    const res = { combos: 9, from: 'a', to: 'b', split: null, dims: [{ key: 'x', values: [1, 2, 3] }, { key: 'y', values: [1, 2, 3] }], defaults: { x: 1, y: 1 },
      cols: ['p.x', 'p.y', 'sharpe', 'trades', 'mar', 'cagr', 'max_dd', 't_stat'], book: [0.1, 50, 0, 0, 0, 1], rows: sh.map((s, k) => [Math.floor(k / 3) + 1, k % 3 + 1, s, 50, s, 0, -0.1, 3]) };
    const an = analyze(res);
    assert.deepStrictEqual(an.robust.p, { x: 2, y: 2 }, 'plateau beats spike');
    assert(an.rows[0].isBook && an.bookPct === 3 / 9);
    assert.deepStrictEqual(monthly(['2020-01-30', '2020-01-31', '2020-02-03', '2020-02-28'], [1, 1.1, 1.0, 1.21]), { '2020-01': 0.10000000000000009, '2020-02': 0.09999999999999987 });
    const g = defaultGrid([{ key: 'a', def: 4, min: 1, max: 15, step: 1 }, { key: 'b', def: 200, min: 100, max: 300, step: 25 }], 40);
    assert(nVals(g.a) * nVals(g.b) <= 40 && (4 - g.a.min) % g.a.step === 0 && (200 - g.b.min) % g.b.step === 0, 'grid fits and keeps the book value');
    assert(tCrit(2) === 4.303 && tCrit(500) === 1.96, 't critical values');
    console.log('optim-core self-check ok');
  }
})(typeof window !== 'undefined' ? window : globalThis);
