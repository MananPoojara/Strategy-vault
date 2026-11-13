import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { VerificationView } from "../src/pages/VerificationView";
import { booksFor, filterRows, loadTracker, statusSlices, strategyHref, reportFolder, type Tracker, type TrackerRow } from "../src/services/verification";

/* Shaped exactly like authors/v1/sync_tracker.py output (a cut-down copy of the 13 Nov workbook). */
const row = (n: number, id: string, author: string, book: string, status: TrackerRow["status"], extra: Partial<TrackerRow> = {}): TrackerRow =>
  ({ n, id, author, book, strategy: `Strategy ${id}`, version: `Version ${id}`, status, ...extra });
const report = { file: "spmt_rsi_3-ALL-verification.xlsx", path: "Z:/Yash Darji/Strategy Vault/Larry Connors/S&P Market Timing Course/spmt_rsi_3-ALL-verification.xlsx" };
const tracker: Tracker = {
  schema: 1, source_mtime: "2026-11-13T12:30:00+05:30", synced_at: "2026-11-13T12:50:57+05:30", snapshot: "Verification Tracker 20261113-123000.xlsx",
  overview: { kpi: { versions: 6, testable: 4, done: 3, todo: 1, cant_test: 2 }, progress: 0.75, notes: ["Done = 1 fully verified + 1 earlier."], focus: "Larry Connors" },
  authors: [
    { author: "Larry Connors", books: 2, versions: 4, testable: 3, done: 2, done_earlier: 1, todo: 0, cant_test: 1, progress: 1 },
    { author: "Larry Williams", books: 1, versions: 2, testable: 1, done: 0, done_earlier: 0, todo: 1, cant_test: 1, progress: 0 },
  ],
  books: [{ book: "S&P Market Timing Course (2007)", versions: 3, testable: 3, done: 3, todo: 0, progress: 1 }],
  statusCounts: [
    { key: "done", label: "✔ Done", count: 2 }, { key: "done_earlier", label: "◐ Done earlier (trades only)", count: 1 },
    { key: "todo", label: "○ To do", count: 1 }, { key: "cant_test", label: "✖ Can't test on site", count: 1 }, { key: "needs_data", label: "✖ Site needs data", count: 1 },
  ],
  results: [{ author: "Larry Connors", book: "S&P Market Timing Course", strategy: "SPMT · Cumulative RSI - RSI-3", id: "spmt_rsi_3", check: "Full (trades + stats)",
    trades: 1113, right: 1113, wrong: 0, to_check: 0, result: "1,113 of 1,113 trades right", checked: "2026-11-13", report }],
  good: [{ strategy: "Rule 1 - Buy 10-day highs", book: "STSTW", index: "NIFTYMIDCAP100", trades: 337, cagr: 0.259414, maxdd: -0.139683, car_mdd: 1.857, sharpe: 1.79,
    id: "ststw_10d_high_buy", link: "http://192.168.4.30:8180/#/s/ststw_10d_high_buy?tk=NIFTYMIDCAP100", report: null }],
  bugs: [{ n: 1, found: "2026-10-28", where: "Backtest page", problem: "Nifty column stops early.", status: "Still open (12-11-2026)" }],
  legend: [{ label: "✔ Done", key: "done", text: "Fully checked." }],
  strategies: [
    row(1, "spmt_rsi_3", "Larry Connors", "S&P Market Timing Course (2007)", "done", { result: "1,113 of 1,113 trades right", checked: "2026-11-13", report }),
    row(2, "spmt_rsi_2", "Larry Connors", "S&P Market Timing Course (2007)", "done"),
    row(3, "spmt_eom_puts", "Larry Connors", "S&P Market Timing Course (2007)", "done_earlier"),
    row(4, "hfm_bond_evening", "Larry Connors", "Investment Secrets (1995)", "cant_test", { result: "needs T-bond futures" }),
    row(5, "lw_oops", "Larry Williams", "Long-Term Secrets", "todo"),
    row(6, "lw_cot", "Larry Williams", "Long-Term Secrets", "needs_data"),
  ],
  runs: { runs: 12, strategies: 1, trades: 1113, right: 1113, wrong: 0, last_verified: "2026-11-13" },
};

it("maps the synced JSON to slices, filters, books and links", () => {
  expect(statusSlices(tracker).map(s => [s.key, s.label, s.count])).toEqual([
    ["done", "Done", 2], ["done_earlier", "Done earlier (trades only)", 1], ["todo", "To do", 1], ["needs_data", "Site needs data", 1], ["cant_test", "Can't test on site", 1],
  ]);
  const ids = (f: Parameters<typeof filterRows>[1]) => filterRows(tracker.strategies, f).map(r => r.id);
  expect(ids({ q: "", author: "", book: "", status: "done_any" })).toEqual(["spmt_rsi_3", "spmt_rsi_2", "spmt_eom_puts"]);
  expect(ids({ q: "T-BOND", author: "", book: "", status: "" })).toEqual(["hfm_bond_evening"]);
  expect(ids({ q: "", author: "Larry Williams", book: "", status: "todo" })).toEqual(["lw_oops"]);
  // focus author: the Overview table; any other author: counted from the list (needs data / can't test are not testable)
  expect(booksFor(tracker, "Larry Connors")).toEqual({ rows: tracker.books, fromOverview: true });
  expect(booksFor(tracker, "Larry Williams")).toEqual({ rows: [{ book: "Long-Term Secrets", versions: 2, testable: 1, done: 0, todo: 1, progress: 0 }], fromOverview: false });
  expect(strategyHref("spmt_rsi_3")).toBe("#/s/spmt_rsi_3");
  expect(strategyHref("ststw_10d_high_buy", tracker.good[0].link)).toBe("#/s/ststw_10d_high_buy?tk=NIFTYMIDCAP100");
  expect(reportFolder(report)).toBe("Z:/Yash Darji/Strategy Vault/Larry Connors/S&P Market Timing Course");
});

it("rejects a missing or unknown tracker file with a plain message", async () => {
  await expect(loadTracker(async () => new Response("", { status: 404 }))).rejects.toThrow("has not been synced yet");
  await expect(loadTracker(async () => new Response(JSON.stringify({ schema: 2 }), { status: 200 }))).rejects.toThrow("unknown format");
});

let root: Root | undefined, host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined; vi.unstubAllGlobals();
});

it("renders freshness, version KPIs beside the site count, results, bugs and a filterable list", async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    if (url === "data/verification-tracker.json") return new Response(JSON.stringify(tracker), { status: 200 });
    if (url.startsWith("/api/app/research/summary")) return new Response(JSON.stringify({ totals: { strategies: 2277 } }), { status: 200 });
    return new Response("{}", { status: 404 });
  }));
  vi.stubGlobal("requestAnimationFrame", (f: () => void) => { f(); return 0; });
  Element.prototype.scrollIntoView = () => undefined;
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<VerificationView />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  const text = host.textContent!;
  expect(text).toContain("Tracker updated 13 Nov 2026, 12:30");
  expect(text).toContain("synced 13 Nov 2026, 12:50");
  expect(text).toMatch(/3 of 4 testable versions done/);
  expect(text).toContain("the site lists 2,277 strategies");
  expect([...host.querySelectorAll(".vt-kpi")].map(k => [k.querySelector("b")!.textContent, k.querySelector("small")!.textContent])).toEqual([["4", "versions"], ["3", "versions"], ["1", "versions"], ["2", "versions"]]);
  expect(text).toContain("spmt_rsi_3-ALL-verification.xlsx");
  expect(text).toContain("Still open (12-11-2026)");
  expect(host.querySelector<HTMLAnchorElement>(".vt-results a")!.getAttribute("href")).toBe("#/s/spmt_rsi_3");
  expect(host.querySelector<HTMLAnchorElement>(".vt-good .vt-open")!.getAttribute("href")).toBe("#/s/ststw_10d_high_buy?tk=NIFTYMIDCAP100");
  expect(host.querySelectorAll(".vt-list tbody tr")).toHaveLength(6);
  // audit reports are text, never links
  expect([...host.querySelectorAll("a")].some(a => /xlsx|file:/.test(a.getAttribute("href") || ""))).toBe(false);
  // clicking an author bar filters the list and shows that author's books (counted from the list)
  const williams = [...host.querySelectorAll<HTMLButtonElement>(".vt-abars button")].find(b => b.textContent!.includes("Larry Williams"))!;
  await act(async () => williams.click());
  expect(host.querySelector(".vt-books h2")!.textContent).toContain("Larry Williams");
  expect([...host.querySelectorAll(".vt-list tbody tr")].map(r => r.querySelector("code")!.textContent)).toEqual(["lw_oops", "lw_cot"]);
  // the status legend lists only that status
  const cant = [...host.querySelectorAll<HTMLButtonElement>(".vt-slices button")].find(b => b.textContent!.includes("Can't test"))!;
  await act(async () => cant.click());
  expect([...host.querySelectorAll(".vt-list tbody tr")].map(r => r.querySelector("code")!.textContent)).toEqual(["hfm_bond_evening"]);
});

it("is open to every signed-in user: no reviewer PIN needed for the Verification tab", async () => {
  const { ResearchPortal } = await import("../src/pages/ResearchPortal");
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    if (url === "data/verification-tracker.json") return new Response(JSON.stringify(tracker), { status: 200 });
    if (url.startsWith("/api/app/research/me")) return new Response(JSON.stringify({ employee: null, admin: false, ready: true }), { status: 200 });
    return new Response(JSON.stringify({ error: "Enter first" }), { status: 401 });
  }));
  history.replaceState(null, "", "/#/research?v=verify");
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<ResearchPortal />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  expect(host.querySelector(".rp-gate")).toBeNull();
  expect(host.textContent).toContain("Tracker updated 13 Nov 2026, 12:30");
  expect(host.textContent).not.toContain("the site lists"); // the reviewer-only summary is optional
  history.replaceState(null, "", "/#/research"); dispatchEvent(new HashChangeEvent("hashchange"));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  expect(host.querySelector(".rp-gate")!.textContent).toContain("Open the verification tracker");
  history.replaceState(null, "", "/");
});
