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

const report = [];
const views = [["desktop-steel", 1440, ""], ["desktop-aqua", 1440, "aqua"], ["desktop-midnight", 1440, "midnight"], ["mobile-aqua", 400, "aqua"]];
for (const [name, width, theme] of views) {
  const before = errors.length, checks = {};
  await send("Emulation.setDeviceMetricsOverride", { width, height: 1000, deviceScaleFactor: 1, mobile: width < 600 });
  await send("Page.navigate", { url: `${base}/#/indicators?tk=NIFTY%2050` });
  checks.library = await waitFor(`!!document.querySelector('[data-lab-library] [data-lab-item]')`);
  await evaluate(theme ? `document.documentElement.setAttribute('data-theme', ${JSON.stringify(theme)})` : `document.documentElement.removeAttribute('data-theme')`);
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
