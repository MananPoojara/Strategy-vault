#!/usr/bin/env node
// Dev only: headless Chrome check of the lab pages over the DevTools protocol (no Playwright needed).
// Usage: node scripts/lab-browser-check.mjs http://127.0.0.1:5174 <out-dir>
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [base = "http://127.0.0.1:5174", out = "."] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "lab-chrome-"));
const chrome = spawn("/usr/bin/google-chrome", ["--headless=new", "--disable-gpu", "--disable-dev-shm-usage", "--no-first-run",
  `--user-data-dir=${profile}`, "--remote-debugging-port=0", "about:blank"], { stdio: "ignore" });
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
const send = (method, params = {}) => new Promise((resolve) => { pending.set(++id, resolve); ws.send(JSON.stringify({ id, method, params })); });
const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true })).result?.result?.value;
const waitFor = async (expression, ms = 60000) => { for (let t = 0; t < ms; t += 250) { if (await evaluate(expression)) return true; await sleep(250); } return false; };
const click = (selector) => evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (el && !el.disabled) { el.click(); return true; } return false; })()`);
await send("Runtime.enable");
await send("Page.enable");

// The app applies the account theme after it boots, so switch through its own runtime (client side only; nothing is saved).
const applyTheme = async (theme) => { await waitFor(`!!window.VaultRuntime`, 10000); await sleep(300); await evaluate(`window.VaultRuntime.changeTheme(${JSON.stringify(theme || "steel")})`); await sleep(200); };
const report = [];
const views = [["desktop-steel", 1440, ""], ["desktop-aqua", 1440, "aqua"], ["desktop-midnight", 1440, "midnight"], ["mobile-aqua", 400, "aqua"]];
for (const [name, width, theme] of views) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height: 1000, deviceScaleFactor: 1, mobile: width < 600 });
  await send("Page.navigate", { url: `${base}/#/indicators?tk=NIFTY%2050` });
  checks.library = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]')`);
  await applyTheme(theme);
  checks.label = await evaluate(`[...document.querySelectorAll('[data-server-label]')].some(e => e.textContent === 'User-defined indicator strategy · simulated · not a book method')`);
  if (name === "desktop-steel") {
    checks.chip = (await click(`[data-lab-item="rsi"]`)) && (await waitFor(`!!document.querySelector('[data-lab-chip]')`));
    // Scoped waits: the saved-strategies 503 notice is already on the page and must not end the wait early.
    checks.run = (await click(`[data-lab-action="run"]`)) && (await waitFor(`!!document.querySelector('[data-lab-result], [data-lab-error]:not([data-lab-saved] *):not([data-lab-optimize] *)')`, 120000));
    checks.runOk = await evaluate(`!!document.querySelector('[data-lab-result]')`);
    checks.optimize = (await click(`[data-lab-dim] input[type=checkbox]`)) && (await click(`[data-lab-action="optimize"]`))
      && (await waitFor(`!!document.querySelector('[data-lab-optimize-result], [data-lab-optimize] [data-lab-error]')`, 300000));
    checks.optimizeOk = await evaluate(`!!document.querySelector('[data-lab-optimize-result]')`);
  }
  if (name === "mobile-aqua") { // many-parameter chip: its last input and remove button must stay on screen
    checks.ribbonChip = (await click(`[data-lab-item="ma_ribbon"]`)) && (await waitFor(`!!document.querySelector('[data-lab-chip]')`));
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
  report.push({ name, checks, pageErrors: errors.slice(before) });
}

// Rules/backtest row: full width before any run, rules | backtest after one. [name, width, height, theme]
const layoutViews = [["1920-steel", 1920, 1080, ""], ["1920-midnight", 1920, 1080, "midnight"], ["1425-steel", 1425, 900, ""], ["1425-midnight", 1425, 900, "midnight"], ["400-steel", 400, 900, ""], ["400-midnight", 400, 900, "midnight"]];
const bad = `(() => {
  const scrolls = (el) => { for (let n = el.parentElement; n; n = n.parentElement) if (/auto|scroll/.test(getComputedStyle(n).overflowX)) return true; return false; };
  return [...document.body.querySelectorAll('*')].filter((el) => { const r = el.getBoundingClientRect(); return r.width > 0 && r.right > innerWidth + 1 && !scrolls(el); }).length === 0 && document.documentElement.scrollWidth <= innerWidth + 1;
})()`;
// Spacing: stacked top-level lab sections sit one house step apart (<= 24px), and the optimization table keeps its numeric columns beside the setting.
const sectionGaps = `(() => {
  const gaps = (host) => { const k = [...host.children].filter((e) => e.offsetHeight > 0 && getComputedStyle(e).position !== 'sticky'); return k.slice(1).map((e, i) => e.getBoundingClientRect().top - k[i].getBoundingClientRect().bottom); };
  const grid = document.querySelector('.lab-grid');
  const stacked = grid && getComputedStyle(grid).gridTemplateColumns.split(' ').length === 1;
  return [...gaps(document.querySelector('.lab-page')), ...gaps(document.querySelector('.lab-main')), ...(stacked ? gaps(grid) : [])];
})()`;
const gapsOk = `(${sectionGaps}).every((g) => g >= 0 && g <= 24)`;
const optimizeCompact = `(() => { const row = document.querySelector('.lab-dims tbody tr'); if (!row) return false; const label = row.children[1].getBoundingClientRect(), ins = [...row.querySelectorAll('input[type=number]')]; return ins.length === 3 && ins[2].getBoundingClientRect().right - label.right <= 400 && !!row.children[5].textContent.trim(); })()`;
const runGroup = `(() => { const ins = [...document.querySelectorAll('.lab-settings input')], b = document.querySelector('[data-lab-action=run]').getBoundingClientRect(), l = ins[ins.length - 1].getBoundingClientRect(); return b.left - l.right <= 40 || b.top > l.bottom; })()`;
const widths = `(() => { const r = document.querySelector('.lab-rules')?.getBoundingClientRect().width ?? 0, m = document.querySelector('.lab-main')?.getBoundingClientRect().width ?? 1; return r / m; })()`;
for (const [name, width, height, theme] of layoutViews) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width < 600 });
  await send("Page.navigate", { url: `${base}/#/indicators?tk=NIFTY%2050` });
  await send("Page.reload");
  checks.library = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]') && !!document.querySelector('.lab-rules')`);
  await applyTheme(theme);
  await sleep(500);
  checks.preFull = await evaluate(`document.querySelector('.lab-two')?.dataset.labLayout === 'full' && !document.querySelector('[data-lab-result]')`);
  const ratio = await evaluate(widths);
  checks.preRulesWidthAtLeast90pct = ratio >= 0.9;
  checks.preNoOverflow = await evaluate(bad);
  checks.preSectionGapsAtMost24 = await evaluate(gapsOk);
  checks.preOptimizeColumnsCompact = await evaluate(optimizeCompact);
  checks.preRunButtonBesideInputs = await evaluate(runGroup);
  checks.preLibraryGroupsClosed = await evaluate(`(() => { const h = [...document.querySelectorAll('.lab-group-h')]; return h.length > 1 && h.every((b) => b.getAttribute('aria-expanded') === 'false'); })()`);
  if (width >= 1400) { // an indicator condition needs ~1000px for one line; in the half-width Entry column it wraps to two (left+operator, right), never three
    checks.preConditionAtMostTwoLines = await evaluate(`[...document.querySelectorAll('.lab-cond')].every((c) => c.getBoundingClientRect().height < 90)`);
  }
  const pre = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  mkdirSync(join(out, "prerun"), { recursive: true });
  writeFileSync(join(out, "prerun", `lab-prerun-${name}.png`), Buffer.from(pre.result.data, "base64"));
  checks.run = (await click(`[data-lab-action="run"]`)) && (await waitFor(`!!document.querySelector('[data-lab-result]')`, 120000));
  checks.postSplit = await evaluate(`document.querySelector('.lab-two')?.dataset.labLayout === 'split'`);
  const post = await evaluate(widths);
  checks.postRulesBesideBacktest = width < 1100 || post < 0.6;
  checks.postNoOverflow = await evaluate(bad);
  checks.postSectionGapsAtMost24 = await evaluate(gapsOk);
  const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
  writeFileSync(join(out, "prerun", `lab-postrun-${name}.png`), Buffer.from(shot.result.data, "base64"));
  report.push({ name: `layout-${name}`, ratios: { pre: ratio, post }, checks, pageErrors: errors.slice(before) });
}
await send("Page.navigate", { url: `${base}/#/yt-strategies` });
report.push({ name: "yt-strategies", checks: { rendered: await waitFor(`!!document.querySelector('[data-lab-page="yt-strategies"] [data-lab-error], [data-lab-page="yt-strategies"] .lab-card, [data-lab-page="yt-strategies"] .empty')`) }, pageErrors: [] });
// Chrome keeps writing its profile until it exits; removing it earlier fails with ENOTEMPTY and hides the report.
ws.close();
if (chrome.exitCode === null && chrome.signalCode === null) {
  const exited = new Promise((resolve) => chrome.once("exit", resolve));
  chrome.kill(); await exited;
}
// Its helper processes outlive the main one for a moment, so retry the removal for a few seconds.
for (let i = 0; i < 20; i++) { try { rmSync(profile, { recursive: true, force: true }); break; } catch { await sleep(250); } }
console.log(JSON.stringify(report, null, 2));
process.exit(report.every((r) => !r.pageErrors.length && Object.values(r.checks).every(Boolean)) ? 0 : 1);
