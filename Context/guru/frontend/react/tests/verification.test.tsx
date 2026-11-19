import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { VerificationView } from "../src/pages/VerificationView";
import {
  booksFor, filterRows, loadTracker, statusSlices, strategyHref, reportFolder, resultGroup, errorStatuses, filterErrors, withNewStatuses,
  type Tracker, type TrackerRow, type TrackerResult, type TrackerError,
} from "../src/services/verification";

/* Shaped exactly like authors/v1/sync_tracker.py output (a cut-down copy of the 13 Nov workbook). */
const row = (n: number, id: string, author: string, book: string, status: TrackerRow["status"], extra: Partial<TrackerRow> = {}): TrackerRow =>
  ({ n, id, author, book, strategy: `Strategy ${id}`, version: `Version ${id}`, status, ...extra });
const report = { file: "spmt_rsi_3-ALL-verification.xlsx", path: "Z:/Yash Darji/Strategy Vault/Larry Connors/S&P Market Timing Course/spmt_rsi_3-ALL-verification.xlsx" };
const result = (strategy: string, checked: string, trades: number, right: number, wrong: number, to_check: number, id: string | null = null): TrackerResult =>
  ({ author: "Larry Connors", book: "Book", strategy, id, trades, right, wrong, to_check, result: `${strategy} result`, checked });
const err = (n: number, status: string, id: string | undefined, problem: string, extra: Partial<TrackerError> = {}): TrackerError =>
  ({ n, status, id, strategy: `Strategy ${id ?? n}`, problem, ...extra });
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
    { key: "other", label: "✖ Mismatch", count: 1 },
  ],
  results: [{ author: "Larry Connors", book: "S&P Market Timing Course", strategy: "SPMT · Cumulative RSI - RSI-3", id: "spmt_rsi_3", check: "Full (trades + stats)",
    trades: 1113, right: 1113, wrong: 0, to_check: 0, result: "1,113 of 1,113 trades right", checked: "2026-11-13", report },
    result("Wrong one", "2026-11-12", 158, 115, 43, 0, "spmt_rsi_2"), result("No exports", "2026-11-12", 0, 0, 0, 0),
    result("Still to check", "2026-11-11", 374, 0, 0, 374), result("Older right", "2026-11-10", 50, 50, 0, 0)],
  errors: [
    err(1, "Explained - site bug", "spmt_rsi_3", "Stats mismatch (Strategy vs Nifty)", { why: "Nifty 50 benchmark stops early", report }),
    err(2, "Open - to investigate", "spmt_rsi_2", "Trade WRONG", { count: 2, reason: "exit # outside the day range ##", index: "NIFTY 50", from: "1995-11-03", settings: "Book settings" }),
    err(3, "Site will not run it", "gone_from_site", "Site will not run it", { reason: "Site produced no exports" }),
    err(4, "Open - to investigate", "spmt_rsi_2", "Trade CHECK", { count: 5 }),
  ],
  errors_summary: [{ status: "Explained - export rounding", strategies: 0, rows: 0 }, { status: "Explained - site bug", strategies: 1, rows: 1 },
    { status: "Open - to investigate", strategies: 1, rows: 2 }, { status: "Site will not run it", strategies: 1, rows: 1 }],
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
    row(7, "lw_mism", "Linda Raschke", "Street Smarts", "other", { label: "✖ Mismatch" }),
  ],
  runs: { runs: 12, strategies: 1, trades: 1113, right: 1113, wrong: 0, last_verified: "2026-11-13" },
};

it("maps the synced JSON to slices, filters, books and links", () => {
  expect(statusSlices(tracker).map(s => [s.key, s.label, s.count])).toEqual([
    ["done", "Done", 2], ["done_earlier", "Done earlier (trades only)", 1], ["todo", "To do", 1], ["needs_data", "Site needs data", 1], ["cant_test", "Can't test on site", 1], ["other", "Mismatch", 1],
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

it("splits latest results into trades right / wrong (or to check) / no trades, newest first", () => {
  const g = (k: string) => tracker.results.filter(r => resultGroup(r) === k).map(r => r.strategy);
  expect(g("right")).toEqual(["SPMT · Cumulative RSI - RSI-3", "Older right"]);
  expect(g("wrong")).toEqual(["Wrong one", "Still to check"]);
  expect(g("none")).toEqual(["No exports"]);
});

it("gives statuses added to the workbook later (sent as other + their text) their own key", () => {
  const t = withNewStatuses(tracker);
  expect(t.strategies.find(r => r.id === "lw_mism")!.status).toBe("mismatch");
  expect(statusSlices(t).find(s => s.key === "mismatch")).toEqual({ key: "mismatch", label: "Mismatch", count: 1 });
  expect(withNewStatuses({ ...tracker, strategies: [row(1, "x", "A", "B", "other", { label: "◔ Pre-check only - full check pending" })] }).strategies[0].status).toBe("precheck");
});

it("orders errors & mismatches open first, counts rows and strategies per status, and searches them", () => {
  expect(errorStatuses(tracker)).toEqual([
    { status: "Open - to investigate", rows: 2, strategies: 1, tone: "open" },
    { status: "Explained - export rounding", rows: 0, strategies: 0, tone: "explained" },
    { status: "Explained - site bug", rows: 1, strategies: 1, tone: "explained" },
    { status: "Site will not run it", rows: 1, strategies: 1, tone: "wontrun" },
  ]);
  expect(filterErrors(tracker, "", "").map(e => e.n)).toEqual([2, 4, 1, 3]);
  expect(filterErrors(tracker, "Site will not run it", "").map(e => e.n)).toEqual([3]);
  expect(filterErrors(tracker, "", "day range nifty").map(e => e.n)).toEqual([2]);
  expect(errorStatuses({ ...tracker, errors: undefined, errors_summary: undefined })).toEqual([]);
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
  expect(host.querySelectorAll(".vt-list:not(.vt-errors) tbody tr")).toHaveLength(7);
  // audit reports are text, never links
  expect([...host.querySelectorAll("a")].some(a => /xlsx|file:/.test(a.getAttribute("href") || ""))).toBe(false);
  // clicking an author bar filters the list and shows that author's books (counted from the list)
  const williams = [...host.querySelectorAll<HTMLButtonElement>(".vt-abars button")].find(b => b.textContent!.includes("Larry Williams"))!;
  await act(async () => williams.click());
  expect(host.querySelector(".vt-books h2")!.textContent).toContain("Larry Williams");
  expect([...host.querySelectorAll(".vt-list:not(.vt-errors) tbody tr")].map(r => r.querySelector("code")!.textContent)).toEqual(["lw_oops", "lw_cot"]);
  // the status legend lists only that status
  const cant = [...host.querySelectorAll<HTMLButtonElement>(".vt-slices button")].find(b => b.textContent!.includes("Can't test"))!;
  await act(async () => cant.click());
  expect([...host.querySelectorAll(".vt-list:not(.vt-errors) tbody tr")].map(r => r.querySelector("code")!.textContent)).toEqual(["hfm_bond_evening"]);
});

it("shows results in three tabs with counts, no Good strategies section, and the errors & mismatches table", async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  vi.stubGlobal("fetch", vi.fn(async (url: string) => url === "data/verification-tracker.json" ? new Response(JSON.stringify(tracker), { status: 200 }) : new Response("{}", { status: 404 })));
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<VerificationView />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  const tabs = () => [...host!.querySelectorAll<HTMLButtonElement>(".vt-seg [role=tab]")];
  expect(tabs().map(b => [b.textContent, b.getAttribute("aria-selected")])).toEqual([["Trades right2", "true"], ["Wrong trades2", "false"], ["No trades1", "false"]]);
  const titles = () => [...host!.querySelectorAll(".vt-results .vt-rtitle")].map(p => p.textContent!.trim());
  expect(titles()).toEqual(["SPMT · Cumulative RSI - RSI-3 Full (trades + stats)", "Older right"]);
  await act(async () => tabs()[1].click());
  expect(titles()).toEqual(["Wrong one", "Still to check"]);
  expect(host.querySelector(".vt-results")!.textContent).toContain("43 wrong");
  expect(host.querySelector(".vt-results")!.textContent).toContain("374 to check");
  await act(async () => tabs()[2].click());
  expect(titles()).toEqual(["No exports"]);
  expect(host.querySelector(".vt-results")!.textContent).toContain("no trades checked");
  // the Good strategies section is gone
  expect(host.textContent).not.toContain("Good strategies");
  expect(host.querySelector(".vt-good")).toBeNull();
  // errors: open first; strategy link only when the ID is on the site; audit report as text
  const erows = () => [...host!.querySelectorAll(".vt-errors tbody tr")];
  expect(erows().map(r => r.querySelector("td")!.textContent)).toEqual(["Open - to investigate", "Open - to investigate", "Explained - site bug", "Site will not run it"]);
  expect(erows()[0].querySelector("a")!.getAttribute("href")).toBe("#/s/spmt_rsi_2");
  expect(erows()[0].textContent).toContain("NIFTY 50 · from 3 Nov 1995 · Book settings");
  expect(erows()[3].querySelector("a")).toBeNull(); // gone_from_site is not in the catalogue
  expect(erows()[2].textContent).toContain("spmt_rsi_3-ALL-verification.xlsx");
  expect([...host.querySelectorAll(".vt-errors a")].some(a => /xlsx|file:/.test(a.getAttribute("href") || ""))).toBe(false);
  const chips = () => [...host!.querySelectorAll<HTMLButtonElement>(".vt-echips button")];
  expect(chips().map(c => [c.textContent, c.disabled])).toEqual([["All 4", false], ["Open - to investigate 2", false], ["Explained - export rounding 0", true], ["Explained - site bug 1", false], ["Site will not run it 1", false]]);
  await act(async () => chips()[4].click());
  expect(erows().map(r => r.querySelector("code")!.textContent)).toEqual(["gone_from_site"]);
  await act(async () => chips()[4].click()); // click again = all
  expect(erows()).toHaveLength(4);
});

it("leaves the errors section out when the JSON has none (synced before the sheet existed)", async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const old = { ...tracker, errors: undefined, errors_summary: undefined };
  vi.stubGlobal("fetch", vi.fn(async (url: string) => url === "data/verification-tracker.json" ? new Response(JSON.stringify(old), { status: 200 }) : new Response("{}", { status: 404 })));
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<VerificationView />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  expect(host.textContent).toContain("Latest results");
  expect(host.querySelector("#vt-errors")).toBeNull();
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
