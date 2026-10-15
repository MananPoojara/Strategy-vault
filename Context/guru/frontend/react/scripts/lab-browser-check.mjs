#!/usr/bin/env node
// Dev only: headless Chrome check of the lab pages over the DevTools protocol (no Playwright needed).
// Usage: node scripts/lab-browser-check.mjs http://127.0.0.1:5174 <out-dir> [all|legacy|customize|phase7]
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const modesFor = (mode) => {
  if (!["all", "legacy", "customize", "phase7"].includes(mode)) throw new Error(`Unknown check mode: ${mode}`);
  return mode === "all" ? ["legacy", "customize", "phase7"] : [mode];
};
if (process.argv[2] === "--self-check") {
  assert.deepEqual(modesFor("all"), ["legacy", "customize", "phase7"]);
  for (const mode of ["legacy", "customize", "phase7"]) assert.deepEqual(modesFor(mode), [mode]);
  assert.throws(() => modesFor("typo"), /Unknown check mode/);
  console.log("Browser check modes: all, legacy, customize and phase7 passed.");
  process.exit(0);
}
const [base = "http://127.0.0.1:5174", out = ".", only = "all"] = process.argv.slice(2);
const modes = modesFor(only), core = modes.includes("legacy");
mkdirSync(out, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "lab-chrome-"));
writeFileSync(join(out, "chrome-profile.txt"), profile + "\n");
const chrome = spawn("/usr/bin/google-chrome", ["--headless=new", "--disable-gpu", "--disable-dev-shm-usage", "--no-first-run",
  `--user-data-dir=${profile}`, "--remote-debugging-port=0", "about:blank"], { stdio: "ignore" });
process.on("exit", () => { if (chrome.exitCode === null && chrome.signalCode === null) chrome.kill(); });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let port;
for (let i = 0; i < 100 && !port; i++) {
  await sleep(100);
  const file = join(profile, "DevToolsActivePort");
  if (existsSync(file)) port = readFileSync(file, "utf8").split("\n")[0];
}
if (!port) { chrome.kill(); throw new Error("Chrome did not start"); }
const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve) => (ws.onopen = resolve));
let id = 0;
const pending = new Map(), errors = [];
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (m.id) { pending.get(m.id)?.(m); pending.delete(m.id); }
  else if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
};
const send = (method, params = {}) => new Promise((resolve, reject) => {
  const requestId = ++id;
  const timer = setTimeout(() => { pending.delete(requestId); reject(new Error(`DevTools ${method} timed out`)); }, 30000);
  pending.set(requestId, (message) => { clearTimeout(timer); if (message.error) reject(new Error(`DevTools ${method}: ${message.error.message}`)); else resolve(message); });
  ws.send(JSON.stringify({ id: requestId, method, params }));
});
const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true })).result?.result?.value;
const waitFor = async (expression, ms = 60000) => {
  for (let t = 0; t < ms; t += 250) {
    try { if (await evaluate(expression)) return true; }
    catch (error) { if (!/Inspected target navigated|Execution context was destroyed|Cannot find context/i.test(error.message)) throw error; }
    await sleep(250);
  }
  return false;
};
const navigate = async (url, reload = false) => {
  const wanted = new URL(url), [route, query = ""] = wanted.hash.split("?");
  // The app canonicalizes spaces and adds the default region to its hash. Compare requested route/values, not serialization.
  const atRoute = `location.origin + location.pathname === ${JSON.stringify(wanted.origin + wanted.pathname)} && location.hash.split('?')[0] === ${JSON.stringify(route)} && ${JSON.stringify([...new URLSearchParams(query)])}.every(([k,v]) => new URLSearchParams(location.hash.split('?')[1] ?? '').get(k) === v)`;
  await send("Page.navigate", { url });
  if (!(await waitFor(atRoute, 10000))) throw new Error(`Navigation did not reach ${url}`);
  // A hash navigation is asynchronous. Reload only after its new route is committed.
  if (reload) {
    await send("Page.reload");
    if (!(await waitFor(`(${atRoute}) && document.readyState === 'complete'`, 10000))) throw new Error(`Reload did not settle at ${url}`);
  }
};
const click = (selector) => evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (el && !el.disabled) { el.click(); return true; } return false; })()`);
const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`;
const TAB_IDS = ["strategy", "optimization", "customize", "saved"];
const tabState = (active) => `(() => {
  const ids = ${JSON.stringify(TAB_IDS)}, panels = ids.map((id) => document.getElementById('lab-panel-' + id));
  const library = document.querySelector('.lab-library-slot');
  return panels.every((p, i) => p && p.hidden === (ids[i] !== ${JSON.stringify(active)}) &&
    (p.getBoundingClientRect().width > 0) === (ids[i] === ${JSON.stringify(active)}) &&
    document.getElementById('lab-tab-' + ids[i])?.getAttribute('aria-selected') === String(ids[i] === ${JSON.stringify(active)})) &&
    !!library && library.hidden === (${JSON.stringify(active)} !== 'strategy');
})()`;
const selectTab = async (id) => {
  if (!TAB_IDS.includes(id)) throw new Error(`Unknown lab tab: ${id}`);
  return (await click(`#lab-tab-${id}`)) && (await waitFor(tabState(id), 5000));
};
// Open the visible Library and category before clicking its item (mobile Library starts folded).
const addLibraryIndicator = async (id) => {
  if (!(await selectTab("strategy"))) return false;
  const sel = `[data-lab-item="${id}"]`;
  const ready = await evaluate(`(() => { const e = ${q(sel)}; if (!e || e.disabled) return false;
    const details = e.closest('[data-lab-library]')?.querySelector(':scope > details'); if (details) details.open = true;
    const h = e.closest('.lab-group')?.querySelector('.lab-group-h'); if (h?.getAttribute('aria-expanded') === 'false') h.click(); return true;
  })()`);
  return ready && (await waitFor(`(${q(sel)}?.getBoundingClientRect().width ?? 0) > 0`, 5000)) && (await click(sel));
};
const firstOf = (...sels) => `(${JSON.stringify(sels)}).map((s) => document.querySelector(s)).find(Boolean)`;
const chooseEl = (el, value) => evaluate(`(() => { const e = ${el}; if (!e || e.disabled) return false;
  if (![...e.options].some((o) => o.value === ${JSON.stringify(value)} && !o.disabled)) return false;
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(e, ${JSON.stringify(value)});
  e.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
const choose = (sel, value) => chooseEl(q(sel), value);
const typeInto = (sel, value) => evaluate(`(() => { const e = ${q(sel)}; if (!e || e.disabled) return false;
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(e, ${JSON.stringify(String(value))});
  e.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
const clickText = (sel, text) => evaluate(`(() => { const b = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.trim() === ${JSON.stringify(text)});
  if (!b || b.disabled) return false; b.click(); return true; })()`);
await send("Runtime.enable");
await send("Page.enable");

// The app applies the account theme after it boots, so switch through its own runtime (client side only; nothing is saved).
const applyTheme = async (theme) => { await waitFor(`!!window.VaultRuntime`, 10000); await sleep(300); await evaluate(`window.VaultRuntime.changeTheme(${JSON.stringify(theme || "steel")})`); await sleep(200); };
const report = [];
const addReport = (entry) => {
  report.push(entry);
  writeFileSync(join(out, "report.json"), JSON.stringify(report, null, 2) + "\n");
  console.error(`Checked ${entry.name}: ${Object.entries(entry.checks).filter(([, ok]) => !ok).map(([key]) => key).join(", ") || "passed"}`);
};
const views = [["desktop-steel", 1440, ""], ["desktop-aqua", 1440, "aqua"], ["desktop-midnight", 1440, "midnight"], ["mobile-aqua", 400, "aqua"]];
for (const [name, width, theme] of core ? views : []) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height: 1000, deviceScaleFactor: 1, mobile: width < 600 });
  await navigate(`${base}/#/indicators?tk=NIFTY%2050`);
  checks.library = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]')`);
  checks.strategyTab = await selectTab("strategy");
  await applyTheme(theme);
  checks.label = await evaluate(`[...document.querySelectorAll('[data-server-label]')].some(e => e.textContent === 'User-defined indicator strategy · simulated · not a book method')`);
  if (name === "desktop-steel") {
    checks.chip = (await addLibraryIndicator("rsi")) && (await waitFor(`!!document.querySelector('[data-lab-chip]')`));
    // Scoped waits: the saved-strategies 503 notice is already on the page and must not end the wait early.
    checks.run = (await click(`[data-lab-action="run"]`)) && (await waitFor(`!!document.querySelector('[data-lab-result], [data-lab-error]:not([data-lab-saved] *):not([data-lab-optimize] *)')`, 120000));
    checks.runOk = await evaluate(`!!document.querySelector('[data-lab-result]')`);
    checks.optimizationTab = await selectTab("optimization");
    checks.optimize = (await click(`[data-lab-dim] input[type=checkbox]`)) && (await click(`[data-lab-action="optimize"]`))
      && (await waitFor(`!!document.querySelector('[data-lab-optimize-result], [data-lab-optimize] [data-lab-error]')`, 300000));
    checks.optimizeOk = await evaluate(`!!document.querySelector('[data-lab-optimize-result]')`);
    checks.strategyResultPreserved = (await selectTab("strategy")) && (await evaluate(`!!document.querySelector('[data-lab-result]')`));
    checks.optimizationResultPreserved = (await selectTab("optimization")) && (await evaluate(`!!document.querySelector('[data-lab-optimize-result]')`));
    checks.returnToStrategy = await selectTab("strategy");
  }
  if (name === "mobile-aqua") { // many-parameter chip: its last input and remove button must stay on screen
    checks.ribbonChip = (await addLibraryIndicator("ma_ribbon")) && (await waitFor(`!!document.querySelector('[data-lab-chip]')`));
  }
  // No element may extend past the viewport unless a horizontal-scroll container holds it.
  checks.noOverflowingElement = await evaluate(`(() => {
    const scrolls = (el) => { for (let n = el.parentElement; n; n = n.parentElement) if (/auto|scroll/.test(getComputedStyle(n).overflowX)) return true; return false; };
    const bad = [...document.body.querySelectorAll('*')].filter((el) => { const r = el.getBoundingClientRect(); return r.width > 0 && r.right > innerWidth + 1 && !scrolls(el); });
    if (bad.length) console.warn(bad.slice(0, 5).map((e) => e.tagName + '.' + e.className).join(' | '));
    return bad.length === 0;
  })()`);
  checks.noHorizontalScroll = await evaluate(`document.documentElement.scrollWidth <= innerWidth + 1`);
  checks.noVerified = await evaluate(`!/\\bverified\\b|TradingView-matched/i.test(document.body.innerText)`);
  const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  writeFileSync(join(out, `lab-${name}.png`), Buffer.from(shot.result.data, "base64"));
  addReport({ name, checks, pageErrors: errors.slice(before) });
}

// Rules/backtest row: full width before any run; rules then full-width backtest after one. [name, width, height, theme]
const layoutViews = [["1920-steel", 1920, 1080, ""], ["1920-midnight", 1920, 1080, "midnight"], ["1425-steel", 1425, 900, ""], ["1425-midnight", 1425, 900, "midnight"], ["400-steel", 400, 900, ""], ["400-midnight", 400, 900, "midnight"]];
const bad = `(() => {
  const scrolls = (el) => { for (let n = el.parentElement; n; n = n.parentElement) if (/auto|scroll/.test(getComputedStyle(n).overflowX)) return true; return false; };
  return [...document.body.querySelectorAll('*')].filter((el) => { const r = el.getBoundingClientRect(); return r.width > 0 && r.right > innerWidth + 1 && !scrolls(el); }).length === 0 && document.documentElement.scrollWidth <= innerWidth + 1;
})()`;
// Spacing: stacked top-level lab sections sit one house step apart (<= 24px), and the optimization table keeps its numeric columns beside the setting.
const sectionGaps = `(() => {
  const gaps = (host) => { const k = [...host.children].filter((e) => e.offsetHeight > 0 && getComputedStyle(e).position !== 'sticky'); return k.slice(1).map((e, i) => e.getBoundingClientRect().top - k[i].getBoundingClientRect().bottom); };
  const panel = document.querySelector('.lab-tab-panel:not([hidden])');
  const grid = document.querySelector('.lab-grid');
  const stacked = grid && getComputedStyle(grid).gridTemplateColumns.split(' ').length === 1;
  return [...gaps(document.querySelector('.lab-page')), ...gaps(document.querySelector('.lab-main')), ...(panel ? gaps(panel) : []), ...(stacked ? gaps(grid) : [])];
})()`;
const gapsOk = `(${sectionGaps}).every((g) => g >= 0 && g <= 24)`;
const optimizeCompact = `(() => { const row = document.querySelector('.lab-dims tbody tr[data-lab-dim]'); if (!row || row.getBoundingClientRect().width <= 0) return false; const label = row.children[1].getBoundingClientRect(), ins = [...row.querySelectorAll('input[type=number]')]; return ins.length === 3 && ins[2].getBoundingClientRect().right - label.right <= 400 && !!row.children[5].textContent.trim(); })()`;
const runGroup = `(() => { const ins = [...document.querySelectorAll('.lab-settings input')], b = document.querySelector('[data-lab-action=run]').getBoundingClientRect(), l = ins[ins.length - 1].getBoundingClientRect(); return b.left - l.right <= 40 || b.top > l.bottom; })()`;
const stackedBacktest = `(() => { const rules = document.querySelector('.lab-rules')?.getBoundingClientRect(), result = document.querySelector('[data-lab-result]')?.getBoundingClientRect(), main = document.querySelector('.lab-main')?.getBoundingClientRect(); return !!rules && !!result && !!main && result.width >= main.width * 0.9 && rules.width >= main.width * 0.9 && result.top >= rules.bottom; })()`;
const widths = `(() => { const r = document.querySelector('.lab-rules')?.getBoundingClientRect().width ?? 0, m = document.querySelector('.lab-main')?.getBoundingClientRect().width ?? 1; return r / m; })()`;
for (const [name, width, height, theme] of core ? layoutViews : []) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width < 600 });
  await navigate(`${base}/#/indicators?tk=NIFTY%2050`, true);
  checks.library = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]') && !!document.querySelector('.lab-rules')`);
  checks.strategyTab = await selectTab("strategy");
  await applyTheme(theme);
  await sleep(500);
  checks.preFull = await evaluate(`document.querySelector('.lab-two')?.dataset.labLayout === 'full' && !document.querySelector('[data-lab-result]')`);
  const ratio = await evaluate(widths);
  checks.preRulesWidthAtLeast90pct = ratio >= 0.9;
  checks.preNoOverflow = await evaluate(bad);
  checks.preSectionGapsAtMost24 = await evaluate(gapsOk);
  checks.preOptimizationTab = await selectTab("optimization");
  checks.preOptimizeColumnsCompact = await evaluate(optimizeCompact);
  checks.preOptimizationNoOverflow = await evaluate(bad);
  checks.preReturnToStrategy = await selectTab("strategy");
  checks.preRunButtonBesideInputs = await evaluate(runGroup);
  checks.preLibraryGroupsClosed = await evaluate(`(() => { const h = [...document.querySelectorAll('.lab-group-h')]; return h.length > 1 && h.every((b) => b.getAttribute('aria-expanded') === 'false'); })()`);
  if (width >= 1400) { // an indicator condition needs ~1000px for one line; in the half-width Entry column it wraps to two (left+operator, right), never three
    checks.preConditionAtMostTwoLines = await evaluate(`[...document.querySelectorAll('.lab-cond')].every((c) => c.getBoundingClientRect().height < 90)`);
  }
  const pre = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  mkdirSync(join(out, "prerun"), { recursive: true });
  writeFileSync(join(out, "prerun", `lab-prerun-${name}.png`), Buffer.from(pre.result.data, "base64"));
  checks.run = (await click(`[data-lab-action="run"]`)) && (await waitFor(`!!document.querySelector('[data-lab-result]')`, 120000));
  checks.postStacked = await evaluate(`document.querySelector('.lab-two')?.dataset.labLayout === 'stacked'`);
  const post = await evaluate(widths);
  checks.postRulesFullWidth = post >= 0.9;
  checks.postBacktestFullWidthBelowRules = await evaluate(stackedBacktest);
  checks.postNoOverflow = await evaluate(bad);
  checks.postSectionGapsAtMost24 = await evaluate(gapsOk);
  const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  writeFileSync(join(out, "prerun", `lab-postrun-${name}.png`), Buffer.from(shot.result.data, "base64"));
  addReport({ name: `layout-${name}`, ratios: { pre: ratio, post }, checks, pageErrors: errors.slice(before) });
}
if (core) {
await navigate(`${base}/#/yt-strategies`);
addReport({ name: "yt-strategies", checks: { rendered: await waitFor(`!!document.querySelector('[data-lab-page="yt-strategies"] [data-lab-error], [data-lab-page="yt-strategies"] .lab-card, [data-lab-page="yt-strategies"] .empty')`) }, pageErrors: [] });
}

// Customize a book method on NIFTY 50: book-grouped cards with plain summaries, then the four steps. [name, width, height, theme]
const customizeViews = [["1920-steel", 1920, 1080, ""], ["1920-midnight", 1920, 1080, "midnight"], ["400-steel", 400, 900, ""], ["400-midnight", 400, 900, "midnight"]];
const setInput = (selector, value) => evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return false;
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(el, ${JSON.stringify(value)}); el.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
const shotOf = async (selector, file) => {
  await evaluate(`scrollTo(0, 0)`); // the sticky top bar would otherwise sit over the clipped section
  await sleep(400);
  const r = await evaluate(`(() => { const b = document.querySelector(${JSON.stringify(selector)})?.getBoundingClientRect(); return b ? { x: b.left + scrollX, y: b.top + scrollY, width: b.width, height: b.height } : null; })()`);
  if (!r || r.width <= 0 || r.height <= 0) return false;
  const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true, clip: { ...r, scale: 1 } });
  writeFileSync(join(out, file), Buffer.from(shot.result.data, "base64"));
  return true;
};
const summaries = {};
for (const [name, width, height, theme] of modes.includes("customize") ? customizeViews : []) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width < 600 });
  await navigate(`${base}/#/indicators?tk=NIFTY%2050`, true);
  checks.customizeTab = await selectTab("customize");
  checks.cards = await waitFor(`!!document.querySelector('[data-lab-method="spmt_rsi2_4"]')`);
  await applyTheme(theme);
  await evaluate(`document.querySelector('[data-lab-customize]').scrollIntoView({ block: 'start' })`);
  checks.bookTitles = await evaluate(`[...document.querySelectorAll('.lab-book > summary b')].some((b) => b.textContent === 'S&P Market Timing Course — Larry Connors')`);
  checks.noRawTypeCodes = await evaluate(`!/individual_stock|index_or_etf|declared_index_timing/.test(document.querySelector('.lab-books').innerText)`);
  checks.stockOnlyGreyed = await evaluate(`(() => { const b = document.querySelector('[data-lab-method="f1a"]'); return !!b && b.disabled && b.textContent.includes('Stocks only — pick a stock to run this'); })()`);
  checks.firstCardSummaries = await waitFor(`(document.querySelector('[data-lab-method] .lab-m-sum')?.textContent ?? '').startsWith('Buy when')`, 30000);
  if (name === "1920-steel") for (const id of ["spmt_rsi2_4", "f1a", "hbqts_profit_target", "hpetf_rsi75_short"]) {
    await evaluate(`document.querySelector('[data-lab-method="${id}"]')?.scrollIntoView({ block: 'center' })`);
    await waitFor(`(document.querySelector('[data-lab-method="${id}"] .lab-m-sum')?.textContent ?? '').trim().length > 0`, 15000);
    summaries[id] = await evaluate(`document.querySelector('[data-lab-method="${id}"] .lab-m-sum')?.textContent`);
  }
  checks.cardsScreenshot = await shotOf("[data-lab-customize]", `customize-cards-${name}.png`);
  checks.pick = (await click(`[data-lab-method="spmt_rsi2_4"]`)) && (await waitFor(`!!document.querySelector('[data-lab-phrases]')`));
  checks.loadedLink = await evaluate(`!!document.querySelector('[data-lab-action="jump-rules"]')`);
  checks.loadedLinkShowsStrategy = (await click('[data-lab-action="jump-rules"]')) && (await waitFor(tabState("strategy"), 5000));
  checks.pickedTemplatePersists = (await selectTab("customize")) && (await evaluate(`!!document.querySelector('[data-lab-phrases]')`));
  checks.aboutCollapsed = await evaluate(`document.querySelector('[data-lab-template] .lab-about')?.open === false`);
  checks.inlineEdit = await setInput(`[data-lab-phrases] input[aria-label="Entry 2 · Threshold"]`, "5");
  await sleep(200);
  checks.inlineEditShowsStrategy = await selectTab("strategy");
  checks.builderReflects = await evaluate(`[...document.querySelectorAll('.lab-rules input[type=number]')].some((i) => i.value === '5') && (document.querySelector('[data-lab-variant-summary]')?.textContent ?? '').includes('RSI(2) is below 5')`);
  checks.inlineEditPersistsInCustomize = (await selectTab("customize")) && (await evaluate(`document.querySelector('[data-lab-phrases] input[aria-label="Entry 2 · Threshold"]')?.value === '5'`));
  checks.run = (await click(`[data-lab-action="customize-run"]`)) && (await waitFor(`!!document.querySelector('[data-lab-result]')`, 120000));
  checks.customizeRunShowsStrategy = await evaluate(tabState("strategy"));
  checks.returnToCustomizeAfterRun = await selectTab("customize");
  checks.runOriginLabel = await evaluate(`!!document.querySelector('[data-lab-action="jump-result"]')`);
  checks.resultLinkShowsStrategy = (await click('[data-lab-action="jump-result"]')) && (await waitFor(tabState("strategy"), 5000));
  checks.returnForFidelity = await selectTab("customize");
  checks.compare = (await click(`[data-lab-action="fidelity"]`)) && (await waitFor(`!!document.querySelector('[data-lab-fidelity-badge], .lab-fidelity, [data-lab-template] [data-lab-error]')`, 120000));
  checks.fidelityLabel = await evaluate(`[...document.querySelectorAll('[data-server-label]')].some((e) => e.textContent === 'Template fidelity · display only · not a verification')`);
  const badge = await evaluate(`document.querySelector('[data-lab-fidelity-badge]')?.textContent ?? document.querySelector('[data-lab-template] [data-lab-error]')?.textContent ?? ''`);
  checks.templateLabel = await evaluate(`[...document.querySelectorAll('[data-server-label]')].some((e) => e.textContent === 'Book template · declared adaptation into lab rules · not a verification')`);
  checks.noOverflow = await evaluate(bad);
  checks.sectionGapsAtMost24 = await evaluate(gapsOk);
  checks.noVerified = await evaluate(`!/\\bverified\\b|TradingView-matched/i.test(document.body.innerText)`);
  checks.stepsScreenshot = await shotOf("[data-lab-customize]", `customize-steps-${name}.png`);
  checks.savePrefill = (await click(`[data-lab-action="customize-save"]`)) && (await waitFor(`document.querySelector('[data-lab-saved] input[aria-label="Strategy name"]')?.value === 'My RSI(2) < 4'`, 5000));
  checks.saveShowsSavedTab = await evaluate(tabState("saved"));
  checks.saveFocused = await evaluate(`document.activeElement?.getAttribute('aria-label') === 'Strategy name'`);
  addReport({ name: `customize-${name}`, badge, checks, pageErrors: errors.slice(before) });
}
if (Object.keys(summaries).length) addReport({ name: "customize-card-summaries", summaries, checks: {}, pageErrors: [] });

const gapsWithin = (sel, max) => `(() => { const box = ${q(sel)}; if (!box) return false;
  const k = [...box.children].filter((e) => e.offsetHeight > 0);
  return k.length > 1 && k.slice(1).every((e, i) => { const g = e.getBoundingClientRect().top - k[i].getBoundingClientRect().bottom; return g >= -1 && g <= ${max}; }); })()`;
const compactControls = `(() => { const c = [...document.querySelectorAll('.lab-rules select, .lab-rules input:not([type=checkbox]):not([type=radio])')].filter((e) => e.offsetHeight > 0); return c.length > 0 && c.every((e) => e.offsetHeight <= 32); })()`;
const indentOk = `(() => { const c = [...document.querySelectorAll('.lab-group-body')]; return c.length > 0 && c.every((b) => { const s = getComputedStyle(b); return parseFloat(s.paddingLeft) + parseFloat(s.marginLeft) <= 16; }); })()`;
const leafTwoLines = `(() => { const c = [...document.querySelectorAll('[data-lab-node][data-lab-kind=cmp]')]; return c.length > 0 && c.every((e) => e.getBoundingClientRect().height < 90); })()`;
const noWhite = `(() => { const c = [...document.querySelectorAll('.lab-page .box, .lab-page select, .lab-page input, .lab-page details')]; return c.length > 0 && c.every((e) => getComputedStyle(e).backgroundColor !== 'rgb(255, 255, 255)'); })()`;
const noEmptyColumns = `(() => { const tables = [...document.querySelectorAll('.lab-page table')].filter((t) => t.offsetHeight > 0); if (!tables.length) return false;
  return tables.every((t) => { const rows = [...t.tBodies].flatMap((b) => [...b.rows]).filter((r) => !r.classList.contains('lab-dim-group')); if (!rows.length) return false;
    const n = Math.max(...rows.map((r) => r.cells.length));
    for (let c = 0; c < n; c++) if (rows.every((r) => !r.cells[c] || (['', '—'].includes(r.cells[c].textContent.trim()) && !r.cells[c].querySelector('input,button,select,details')))) return false;
    return true; }); })()`;
const firstEntry = '[data-lab-list="entry.all"] > [data-lab-kind="cmp"]';
// Missing controls are failures, including while Phase 7 is being assembled; they never count as optional passes.
const probe = (expression) => evaluate(`(() => { try { return !!(${expression}); } catch { return false; } })()`);
const screenshot = async (file) => {
  const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  writeFileSync(join(out, "p7", file), Buffer.from(shot.result.data, "base64"));
};
for (const [name, width, height, theme] of modes.includes("phase7") ? layoutViews : []) {
  const before = errors.length, checks = {}, failures = {};
  const check = async (key, run) => {
    try { checks[key] = !!(await run()); } catch (error) { checks[key] = false; failures[key] = error.message; }
    return checks[key];
  };
  const action = (key, act, expected) => check(key, async () => (await act()) && (await waitFor(`(() => { try { return !!(${expected}); } catch { return false; } })()`, 5000)));
  const load = async () => {
    await navigate(`${base}/#/indicators?tk=NIFTY%2050`, true);
    const ready = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]') && !!document.querySelector('.lab-rules')`);
    const strategy = await selectTab("strategy");
    await applyTheme(theme);
    return ready && strategy;
  };
  mkdirSync(join(out, "p7"), { recursive: true });
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width < 600 });
  await check("p7Ready", load);
  await action("p7BothPanel", () => clickText('[role=group][aria-label="Direction"] button', "Both"), `!!${q('[data-lab-side="short"]')}`);
  await action("p7GroupAdded", () => choose('[data-lab-list="entry.all"] > select[data-lab-add]', "any"), `!!${q('[data-lab-list="entry.all"] [data-lab-kind="any"]')}`);
  await action("p7RangeOp", () => choose(`${firstEntry} select[aria-label="Operator"]`, "between"), `!!${q('[data-lab-list="entry.all"] [data-lab-kind="range"]')}`);
  await action("p7Formula", () => chooseEl(`(${firstOf('[data-lab-list="entry.all"]', '.lab-cond')})?.querySelector('select[aria-label="Operand type"]')`, "ref"), `!!${q('.lab-operand[data-lab-operand-kind="ref"]')}`);
  await action("p7StopOrder", () => clickText('[role=group][aria-label="Long entry order"] button', "Stop"), `[...document.querySelectorAll('[data-lab-orders="long"] select')].some((e) => e.value === 'pct')`);
  await action("p7TakeProfit", () => choose('select[aria-label="Add exit (long)"]', "take_profits"), `!!${q('[data-lab-orders="long"] [data-lab-exit="take_profits"]')}`);
  await action("p7RiskOpen", () => click('details[data-lab-risk] > summary'), `${q('details[data-lab-risk]')}?.open`);
  await check("p7MoneyBlockedOnIndex", () => probe(`[...document.querySelectorAll('[role=group][aria-label="Results in"] button')].some((e) => e.textContent.trim() === 'Money' && e.disabled) && ${q('[data-lab-money-blocked]')}?.textContent.includes('an index is never priced')`));
  await check("p7StrategyTabVisibility", () => probe(tabState("strategy")));
  await check("p7Caps", () => probe(`/comparisons \\d+\\/40/.test(${q('[data-lab-caps]')}?.textContent ?? '')`));
  const ruleState = await evaluate(`JSON.stringify([...document.querySelectorAll('.lab-rules input, .lab-rules select')].map((e) => [e.getAttribute('aria-label'), e.value, e.checked]))`);
  await check("p7OptimizationTabVisibility", () => selectTab("optimization"));
  await check("p7OptimizationNoOverflow", () => probe(bad));
  await check("p7RulesSurviveTabSwitch", async () => (await selectTab("strategy")) && (await evaluate(`JSON.stringify([...document.querySelectorAll('.lab-rules input, .lab-rules select')].map((e) => [e.getAttribute('aria-label'), e.value, e.checked]))`)) === ruleState);
  const layoutChecks = {
    p7RulesFullWidth: `(${widths}) >= 0.9`, p7NoOverflow: bad, p7PageGaps: gapsOk,
    p7RulesRhythm: gapsWithin(".lab-rules", 15), p7SideRhythm: gapsWithin('[data-lab-side="long"]', 11),
    p7CompactControls: compactControls, p7Indent: indentOk,
    p7LibraryClosed: `(() => { const h = [...document.querySelectorAll('[data-lab-library] .lab-group-h')]; return h.length > 1 && h.every((e) => e.getAttribute('aria-expanded') === 'false'); })()`,
  };
  if (width >= 1400) layoutChecks.p7LeafTwoLines = leafTwoLines;
  if (theme === "midnight") layoutChecks.p7NoWhiteInMidnight = noWhite;
  for (const [key, expression] of Object.entries(layoutChecks)) await check(key, () => probe(expression));
  if (!checks.p7RulesRhythm) failures.rulesGaps = await evaluate(`(() => { const k = [...document.querySelector('.lab-rules').children].filter((e) => e.offsetHeight > 0); return k.slice(1).map((e,i) => ({element: e.className, previous: k[i].className, gap: e.getBoundingClientRect().top-k[i].getBoundingClientRect().bottom})); })()`);
  await check("p7PreScreenshot", async () => { await screenshot(`p7-prerun-${name}.png`); return true; });
  await check("p7Run", async () => (await click('[data-lab-action="run"]')) &&
    (await waitFor(`!!document.querySelector('[data-lab-result], .lab-two [data-lab-error]')`, 120000)) &&
    (await probe(`!!${q('[data-lab-result]')}`)));
  await check("p7PostStackedFullWidth", () => probe(`${q(".lab-two")}?.dataset.labLayout === 'stacked' && (${stackedBacktest})`));
  await check("p7IndexNote", () => probe(`${q('[data-lab-execution-notes]')}?.textContent.includes('An index cannot be traded')`));
  await check("p7ShortTrades", () => probe(`[...document.querySelectorAll('table[data-lab-trades] td')].some((e) => e.textContent.trim() === 'Short')`));
  await check("p7NoEmptyColumns", () => probe(noEmptyColumns));
  if (!checks.p7NoEmptyColumns) failures.emptyColumns = await evaluate(`(() => { return [...document.querySelectorAll('.lab-page table')].filter((t) => t.offsetHeight > 0).map((t) => { const rows = [...t.tBodies].flatMap((b) => [...b.rows]).filter((r) => !r.classList.contains('lab-dim-group')); const empty = []; const n=Math.max(0,...rows.map((r)=>r.cells.length)); for(let c=0;c<n;c++) if(rows.every((r)=>!r.cells[c]||(['','—'].includes(r.cells[c].textContent.trim())&&!r.cells[c].querySelector('input,button,select,details')))) empty.push(c); return {table:t.className,columns:empty,head:t.tHead?.innerText}; }).filter((t)=>t.columns.length); })()`);
  await check("p7PostNoOverflow", () => probe(bad));
  await check("p7PostGaps", () => probe(gapsOk));
  if (!checks.p7Run) failures.run = await evaluate(`({ rules: document.querySelector('.lab-rules .rnote')?.innerText ?? '', error: document.querySelector('.lab-two [data-lab-error]')?.innerText ?? '' })`);
  await check("p7PostScreenshot", async () => { await screenshot(`p7-postrun-${name}.png`); return true; });
  if (name === "1920-steel") {
    await check("p7CalendarReady", load);
    await action("p7CalendarOperand", () => choose(`${firstEntry} select[aria-label="Operand type"]`, "calendar"), `!!${q(`${firstEntry} select[aria-label="Calendar value"]`)}`);
    await check("p7CalendarValue", () => choose(`${firstEntry} select[aria-label="Calendar value"]`, "sessions_left.m"));
    await action("p7CalendarNumber", () => chooseEl(`${q(firstEntry)}?.querySelectorAll(':scope > .lab-operand')[1]?.querySelector('select[aria-label="Operand type"]')`, "number"), `!!${q(`${firstEntry} > .lab-operand[data-lab-operand-kind="number"] input`)}`);
    await check("p7CalendarThreshold", () => typeInto(`${firstEntry} > .lab-operand[data-lab-operand-kind="number"] input`, 5));
    await check("p7CalendarOperator", () => choose(`${firstEntry} select[aria-label="Operator"]`, "<"));
    await check("p7Adaptations", async () => (await click('[data-lab-action="run"]')) &&
      (await waitFor(`!!document.querySelector('[data-lab-result], .lab-two [data-lab-error]')`, 120000)) &&
      (await probe(`${q('[data-lab-adaptations]')}?.textContent.includes('sessions_left')`)));
    await check("p7AdaptationsScreenshot", async () => { await screenshot(`p7-adaptations-${name}.png`); return true; });
    await action("p7OptimizeGroup", () => choose('[data-lab-list="entry.all"] > select[data-lab-add]', "any"), `!!${q('[data-lab-list="entry.all"] [data-lab-kind="any"]')}`);
    await check("p7OptimizeTab", () => selectTab("optimization"));
    await check("p7OptimizeDimension", () => click('tr[data-lab-dim="entry.all[1].any[0].rhs.p.length"] input[type=checkbox]'));
    await check("p7OptimizeNested", async () => checks.p7OptimizeDimension && (await click('[data-lab-action="optimize"]')) &&
      (await waitFor(`!!document.querySelector('[data-lab-optimize-result], [data-lab-optimize] [data-lab-error]')`, 300000)) &&
      (await probe(`!!${q('[data-lab-optimize-result]')}`)));
    await check("p7OptimizeScreenshot", async () => { await screenshot(`p7-optimize-${name}.png`); return true; });
  }
  addReport({ name: `phase7-${name}`, checks, failures, pageErrors: errors.slice(before) });
}
// Profiles are retained for inspection; stop only the Chrome process started by this check.
ws.close();
if (chrome.exitCode === null && chrome.signalCode === null) {
  const exited = new Promise((resolve) => chrome.once("exit", resolve));
  chrome.kill(); await exited;
}
writeFileSync(join(out, "chrome-profile.txt"), profile + "\n");
console.error(`Chrome profile retained: ${profile}`);
console.log(JSON.stringify(report, null, 2));
process.exit(report.every((r) => !r.pageErrors.length && Object.values(r.checks).every(Boolean)) ? 0 : 1);
