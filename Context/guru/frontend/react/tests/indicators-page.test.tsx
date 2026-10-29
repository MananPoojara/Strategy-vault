import {portfolioCatalogueFixture,portfolioResultFixture} from "./fixtures/portfolio";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import {
  catalogueFixture, catalogue7Fixture, fetchRouter, flush, glInstrumentsFixture, jsonResponse, mount, optimizeFixture, publishedFixture, runFixture,
  seriesFixture, templateFixture, templatesFixture, tickersFixture, unavailableTemplateFixture, unmount, pickIndicator,
} from "./fixtures/lab";

const me = (role: string) => jsonResponse({ user: { id: "u1", name: "N", email: "n@x", role }, settings: { theme: "aqua" } });
const series = (url: string) => { const id = new URL(url, "http://x").searchParams.get("id")!; return jsonResponse(seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] })); };
let catalogueAnswer = catalogueFixture;
let runAnswer: () => Response | Promise<Response>;
let usTickers = tickersFixture;
let templateAnswer = templateFixture;
const routes = (role = "researcher") => fetchRouter([
  ["/api/app/me", () => me(role)],
  ["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)],
  ["/api/app/lab/strategies/review", () => jsonResponse([])],
  ["/api/app/lab/strategies", () => jsonResponse({ error: "Saved strategies need the auth service; this local dev proxy has none." }, 503)],
  [/\/api\/(in|us|gl)\/lab\/catalogue/, () => jsonResponse(catalogueAnswer)],
  ["/api/us/tickers", () => jsonResponse(usTickers)],
  ["/api/in/tickers", () => jsonResponse(tickersFixture)],
  ["/api/gl/lab/instruments", () => jsonResponse(glInstrumentsFixture)],
  [/\/lab\/series/, (url) => series(url)],
  [/\/lab\/templates/, () => jsonResponse(templatesFixture)],
  [/\/lab\/template\?/, () => jsonResponse(templateAnswer)],
  [/\/lab\/run/, () => runAnswer()],
  [/\/lab\/optimize/, () => jsonResponse(optimizeFixture)],
]);
beforeEach(() => { catalogueAnswer = catalogueFixture; clearLabCache(); runAnswer = () => jsonResponse(runFixture); usTickers = tickersFixture; templateAnswer = templateFixture; });
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); history.replaceState(null, "", "/"); });
/** A new strategy starts with no rules; these tests begin from the old starter rows instead: close above EMA 50, plus
 *  "bars held ≥ 5" on an engine without the rule tree (the rule tree falls back to the default exit). */
const starterRows = async (host: HTMLElement) => {
  const tree = host.querySelector<HTMLButtonElement>('[data-lab-list="entry.all"] [data-lab-add-cond]');
  if (tree) return act(async () => tree.click());
  for (const b of host.querySelectorAll<HTMLButtonElement>(".lab-side > button")) await act(async () => b.click());
};
const go = async (hash: string, role = "researcher", { empty = false } = {}) => {
  history.replaceState(null, "", `/${hash}`);
  const f = routes(role);
  vi.stubGlobal("fetch", f);
  const host = await mount(<IndicatorsPage />);
  await flush(); await flush();
  if (!empty && !/[?&](customize|strategy|published)=/.test(hash)) await starterRows(host);
  return { host, f };
};
const pick = async (host: HTMLElement, tk: string) => {
  await act(async () => host.querySelector<HTMLButtonElement>("#lab-tab-chart")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Symbol search"]')!.click());
  const dialog = host.querySelector('[role="dialog"][aria-label="Symbol search"]')!;
  const search = dialog.querySelector<HTMLInputElement>('[aria-label="Search instruments"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, tk);
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const result = [...dialog.querySelectorAll<HTMLButtonElement>('.lab-symbol-results button')].find(button => button.querySelector('b')?.textContent === tk)!;
  expect(result.disabled).toBe(false);
  await act(async () => result.click());
  await flush();
  expect(host.querySelector('[role="dialog"][aria-label="Symbol search"]')).toBeNull();
  expect(host.querySelector('[aria-label="Symbol search"]')?.textContent).toContain(tk);
};
/** A portfolio cannot read since-entry values: its exit is an indicator row (Exit card › Indicator). */
const exitRow = async (host: HTMLElement) => act(async () => host.querySelector<HTMLButtonElement>('[data-lab-exit="indicator"]')!.click());
/** There is no hidden default exit: a run without one first picks N candle in the Exit card. */
const run = async (host: HTMLElement) => {
  if (host.textContent?.includes("Pick an exit in the Exit card.")) await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-exit="ncandle"]')!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.click()); await flush();
};

it("renders the catalogue label, library and default instrument; saved-strategy errors stay local", async () => {
  const { host } = await go("#/indicators");
  expect(host.querySelector('[data-lab-page="indicators"]')).not.toBeNull();
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(catalogueFixture.label);
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(0);
  await act(async () => [...host.querySelectorAll(".lab-chart-actions button")].find((b) => b.textContent === "Indicators")!.dispatchEvent(new MouseEvent("click", { bubbles:true })));
  expect(host.querySelectorAll("[data-lab-item]")).toHaveLength(catalogueFixture.indicators.length);
  expect(host.querySelector('[aria-label="Symbol search"]')?.textContent).toBe("NIFTY 50");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).toContain("auth service");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).toContain("Could not load");
  expect(host.querySelector("[data-lab-saved] [data-lab-error]")?.textContent).not.toContain("Not saved");
});

it("an unknown ?tf= falls back to daily once the catalogue loads", async () => {
  await go("#/indicators?tf=Q");
  expect(location.hash).not.toContain("tf=");
});

it("the chart is its own tab; Strategy & backtest opens on Your idea, with Your idea, Settings and Results as tabs; a run opens Results", async () => {
  catalogueAnswer = catalogue7Fixture;
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => [t.textContent, t.getAttribute("aria-selected")])).toEqual([
    ["Chart", "true"], ["Strategy & backtest", "false"], ["Customize", "false"], ["Saved strategies", "false"]]); // Optimize sits beside Run backtest
  expect(host.querySelector<HTMLElement>("#lab-panel-chart")!.hidden).toBe(false);
  expect(host.querySelector<HTMLElement>("#lab-panel-strategy")!.hidden).toBe(true);
  expect(host.querySelector(".lab-library-slot")).toBeNull(); // library opens on demand without taking chart space
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-build]")!.click()); // Build a strategy ›
  expect(host.querySelector<HTMLElement>("#lab-panel-strategy")!.hidden).toBe(false);
  const steps = [...host.querySelectorAll<HTMLButtonElement>("[data-lab-steps] button")];
  expect(steps.map((b) => b.textContent)).toEqual(["1Your idea", "2Settings", "3Results"]);
  expect(steps[0].getAttribute("aria-current")).toBe("step");
  const panel = (i: number) => host.querySelectorAll<HTMLElement>("#lab-panel-strategy .lab-step")[i];
  expect(panel(0).hidden).toBe(false);
  expect(panel(1).hidden && panel(2).hidden).toBe(true);
  expect(panel(0).querySelector(".lab-rules [data-lab-list]")).not.toBeNull();
  expect(panel(1).querySelector('[aria-label="Analysis settings"]')).not.toBeNull(); // settings live in step 2
  expect(panel(0).querySelector('[aria-label="Analysis settings"]')).toBeNull();
  // Entry, Filters and Exit side by side; a plain condition is a parameter list with its timeframe; More options opens every control
  expect([...panel(0).querySelectorAll("[data-lab-card]")].map((c) => c.getAttribute("data-lab-card"))).toEqual(["entry", "filters", "exit"]);
  const param = panel(0).querySelector<HTMLElement>("[data-lab-param]")!;
  expect(param.querySelector<HTMLSelectElement>('select[aria-label="Timeframe"]')!.value).toBe("D");
  expect(param.querySelector("[data-lab-node-menu]")).toBeNull();
  expect(param.querySelector('input[aria-label="Entry 1 check bars ago"]')).toBeNull(); // the extras sit behind More
  await act(async () => param.querySelector<HTMLButtonElement>("[data-lab-param-more]")!.click());
  expect(param.querySelector<HTMLInputElement>('input[aria-label="Entry 1 check bars ago"]')!.value).toBe("0");
  expect(panel(0).querySelector("[data-lab-caps]")).toBeNull(); // counters only near a limit
  expect(panel(2).querySelector("[data-lab-no-result]")).not.toBeNull();
  let release!: (r: Response) => void;
  runAnswer = () => new Promise<Response>((resolve) => { release = resolve; });
  await run(host);
  expect(panel(2).hidden).toBe(false);
  expect(panel(2).querySelector("[role=status]")?.textContent).toBe("Running backtest…");
  await act(async () => release(jsonResponse(runFixture)));
  await flush();
  expect(panel(2).querySelector("[data-lab-result]")).not.toBeNull();
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-see-chart]")!.click()); // the trades are drawn on the chart
  expect(host.querySelector<HTMLElement>("#lab-panel-chart")!.hidden).toBe(false);
});

it("a failed run clears the previous result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull();
  const tables = host.querySelector("[data-lab-result-tables]")!;
  expect(tables).not.toBeNull();
  expect(tables.closest("[data-lab-results-tabs]")).not.toBeNull(); // every result view is a tab of the Results box under Run backtest
  const tabs = [...host.querySelectorAll<HTMLElement>('[data-lab-results-tabs] [role=tab]')].map((t) => t.textContent);
  expect(tabs).toEqual(["Trades", "Stats & equity drawdown", "YOY-Monthly", "Scan / Explore"]); // no Monte Carlo; stats and charts are one tab
  expect(host.querySelector("[data-lab-step-results] [data-lab-result]")).not.toBeNull();
  runAnswer = () => jsonResponse({ status: "needs_data", error: "NIFTY 50: invalid OHLC bar on 2001-03-02", label: catalogueFixture.label }, 400);
  await run(host);
  expect(host.querySelector("[data-lab-result]")).toBeNull();
  expect(host.querySelector("[data-lab-result-tables]")).toBeNull();
  expect(host.textContent).toContain("NIFTY 50: invalid OHLC bar on 2001-03-02");
});

it("gl shows the provisional strip and hides Customize", async () => {
  const { host } = await go("#/indicators?region=gl");
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("Provisional · TradingView export CRYPTO:BTCUSD, daily");
  expect(host.textContent).toContain("Customize runs on India and US book methods only.");
});

it("viewers cannot optimize", async () => {
  const { host } = await go("#/indicators", "viewer");
  expect(host.textContent).toContain("Researcher access is required for optimization.");
  expect(host.querySelector('[data-lab-action="optimize"]')).toBeNull();
});

it("customize loads the template and runs with its origin", async () => {
  const { host, f } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  expect(host.textContent).toContain("Book template · declared adaptation into lab rules · not a verification");
  await run(host);
  const runCall = f.calls.find((c) => c.url.includes("/lab/run"))!;
  const spec = JSON.parse(new URL(runCall.url, "http://x").searchParams.get("spec")!);
  expect(spec.origin).toEqual({ book: "ststw", method: "rsi2_4", template_version: 1 });
});

it("customize: an inline number edits the shared rules, runs with the origin, and step 4 prefills the save name", async () => {
  const { host, f } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  const inline = host.querySelector<HTMLInputElement>('[data-lab-phrases] input[aria-label="Entry 1 · Threshold"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(inline, "3");
    inline.dispatchEvent(new Event("input", { bubbles: true }));
  });
  // The Strategy rules builder above shows the same number: one spec.
  expect([...host.querySelectorAll<HTMLInputElement>(".lab-rules input[type=number]")].some((i) => i.value === "3")).toBe(true);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="customize-run"]')!.click());
  await flush();
  const spec = JSON.parse(new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x").searchParams.get("spec")!);
  expect(spec.entry.all[0].rhs).toEqual({ value: 3 });
  expect(spec.origin).toEqual({ book: "ststw", method: "rsi2_4", template_version: 1 });
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="customize-save"]')!.click());
  const name = host.querySelector<HTMLInputElement>('[data-lab-saved] input[aria-label="Strategy name"]')!;
  expect(name.value).toBe("My RSI 2 · 4");
  expect(document.activeElement).toBe(name);
});

const spy = { ticker: "SPY", first: "2000-01-03", last: "2026-10-14", has_volume: true, series: [] };

it("a published snapshot opens as an unsaved draft on its own market and instrument", async () => {
  usTickers = [...tickersFixture, spy];
  const { host, f } = await go("#/indicators?published=12");
  expect(host.querySelector('[aria-label="Symbol search"]')?.textContent).toBe("SPY");
  expect(host.querySelector("[data-lab-swap]")).toBeNull();
  await run(host);
  const url = new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x");
  expect(url.pathname).toBe("/api/us/lab/run");
  expect(url.searchParams.get("ticker")).toBe("SPY");
  expect(JSON.parse(url.searchParams.get("spec")!).origin.method).toBe("rsi2_4");
  expect(host.querySelector('[data-lab-action="save"]')).toBeNull();
});

it("a snapshot whose instrument is missing says which one replaced it", async () => {
  const { host } = await go("#/indicators?published=12");
  expect(host.querySelector('[aria-label="Symbol search"]')?.textContent).toBe("NIFTY 50");
  expect(host.querySelector("[data-lab-swap]")?.textContent).toBe("SPY is not available here; showing NIFTY 50.");
  await pick(host, "NIFTYBANK");
  expect(host.querySelector("[data-lab-swap]")).toBeNull();
});

it("changing the instrument clears the run result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull();
  await pick(host, "NIFTYBANK");
  expect(host.querySelector("[data-lab-result]")).toBeNull();
});

it("a run answer that arrives after the instrument changed is dropped", async () => {
  let answer!: (r: Response) => void;
  runAnswer = () => new Promise<Response>((r) => { answer = r; });
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await run(host);
  await pick(host, "NIFTYBANK");
  await act(async () => answer(jsonResponse(runFixture)));
  await flush();
  expect(host.querySelector("[data-lab-result]")).toBeNull();
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.textContent).toBe("Run backtest");
});

it("changing the instrument clears the optimization result", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await act(async () => host.querySelector<HTMLInputElement>("[data-lab-dim] input[type=checkbox]")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.querySelector("[data-lab-optimize-result]")).not.toBeNull();
  await pick(host, "NIFTYBANK");
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
});

it.each([["From", "01-01-2010"], ["To", "01-01-2020"], ["Fees (bps / side)", "4"], ["Slippage (bps / side)", "2"]])("changing %s clears optimization results and their downloads", async (label, value) => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  await act(async () => host.querySelector<HTMLInputElement>("[data-lab-dim] input[type=checkbox]")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.querySelector("[data-lab-optimize-result]")).not.toBeNull();
  const input = host.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
  expect(host.querySelector("[data-lab-tradesheets]")).toBeNull();
});

it("an unavailable template leaves the open snapshot named in the URL", async () => {
  templateAnswer = unavailableTemplateFixture;
  usTickers = [...tickersFixture, spy];
  const { host } = await go("#/indicators?published=12");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-method="rsi2_4"]')!.click());
  await flush();
  expect(host.textContent).toContain(unavailableTemplateFixture.status === "unavailable" ? unavailableTemplateFixture.reason : "");
  expect(location.hash).toContain("published=12");
  expect(location.hash).toContain("customize=rsi2_4");
});


it("Phase7 blocks money on an index before any calculation request", async () => {
  catalogueAnswer = catalogue7Fixture;
  const { host, f } = await go("#/indicators?tk=NIFTY%2050");
  const money = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find((b) => b.textContent === "Cash amounts")!;
  expect(money.disabled).toBe(true);
  expect(host.querySelector("[data-lab-money-blocked]")!.textContent).toContain("index");
  expect(f.calls.some((c) => c.url.includes("/lab/run"))).toBe(false);
});
it("Phase7 money requests omit legacy cost and slippage transport", async () => {
  catalogueAnswer = { ...catalogue7Fixture, execution: { ...catalogue7Fixture.execution!, sizing: [...catalogue7Fixture.execution!.sizing, "units", "value"] } };
  usTickers = [...tickersFixture, { ticker: "SPY", native: true, asset_type: "etf", has_volume: true } as typeof tickersFixture[number]];
  const { host, f } = await go("#/indicators?region=us&tk=SPY");
  const money = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find((b) => b.textContent === "Cash amounts")!;
  await act(async () => money.click());
  await flush();
  expect(host.querySelector('input[aria-label="Fees (bps / side)"]')).toBeNull();
  await run(host);
  const url = new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x");
  expect(url.searchParams.has("cost")).toBe(false);
  expect(url.searchParams.has("slippage")).toBe(false);
  expect(JSON.parse(url.searchParams.get("spec")!).execution.capital).toBe(1_000_000);
});

it("Phase7 nested constant comparisons block the request without losing the loaded template", async () => {
  catalogueAnswer = catalogue7Fixture;
  if (templateFixture.status !== "available") throw new Error("template fixture required");
  templateAnswer = { ...templateFixture, template: { ...templateFixture.template, spec: { ...templateFixture.template.spec,
    entry: { all: [{ any: [{ lhs: { value: 1 }, op: ">", rhs: { value: 2 } }] }] } } } };
  const { host, f } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  expect(host.textContent).toContain("one side must be a series");
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  expect(f.calls.some((c) => c.url.includes("/lab/run"))).toBe(false);
});


it("workspace tabs separate optimization and customize while keeping edits and results", async () => {
  const { host } = await go("#/indicators?tk=NIFTY%2050");
  const panel = (id: string) => host.querySelector<HTMLElement>(`#lab-panel-${id}`)!;
  const choose = async (id: string) => { await act(async () => host.querySelector<HTMLButtonElement>(`#lab-tab-${id}`)!.click()); };
  expect(panel("chart").hidden).toBe(false); // the page opens on the chart
  await choose("strategy");
  expect(panel("strategy").hidden).toBe(false);
  expect(panel("optimization").hidden).toBe(true);
  expect(panel("customize").hidden).toBe(true);
  expect(host.querySelector("#lab-tab-optimization")).toBeNull(); // no Optimization tab: the Optimize button beside Run backtest opens it
  await run(host);
  const result = host.querySelector("[data-lab-result]");
  await choose("strategy");
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-open-optimize]")!.click());
  expect(panel("strategy").hidden).toBe(true);
  expect(panel("optimization").hidden).toBe(false);
  const checked = host.querySelector<HTMLInputElement>("[data-lab-dim] input[type=checkbox]")!;
  await act(async () => checked.click());
  await choose("customize");
  expect(panel("customize").hidden).toBe(false);
  expect(host.querySelector(".lab-library-slot")).toBeNull();
  await choose("strategy");
  expect(host.querySelector("[data-lab-result]")).toBe(result);
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-open-optimize]")!.click());
  expect(checked.checked).toBe(true);
  await act(async () => host.querySelector("#lab-tab-strategy")!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(host.querySelector("#lab-tab-customize")?.getAttribute("aria-selected")).toBe("true");
  expect(document.activeElement).toBe(host.querySelector("#lab-tab-customize"));
});

it("customize jump and save actions open their destination tabs", async () => {
  const { host } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  await act(async () => host.querySelector<HTMLButtonElement>("#lab-tab-customize")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="jump-rules"]')!.click());
  expect(host.querySelector("#lab-tab-strategy")?.getAttribute("aria-selected")).toBe("true");
  await vi.waitFor(() => expect(document.activeElement).toBe(host.querySelector(".lab-rules")));
  await run(host);
  await act(async () => host.querySelector<HTMLButtonElement>("#lab-tab-customize")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="jump-result"]')!.click());
  await vi.waitFor(() => expect(document.activeElement).toBe(host.querySelector("[data-lab-result]")));
  await act(async () => host.querySelector<HTMLButtonElement>("#lab-tab-customize")!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="customize-save"]')!.click());
  expect(host.querySelector("#lab-tab-saved")?.getAttribute("aria-selected")).toBe("true");
  expect((host.querySelector('[data-lab-saved] input[aria-label="Strategy name"]') as HTMLInputElement).value).toBe("My RSI 2 · 4");
});


it("a book variant can replace an indicator through the full editor and retain its origin", async () => {
  const { host, f } = await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  await act(async () => host.querySelector<HTMLButtonElement>("#lab-tab-customize")!.click());
  const edit = host.querySelector<HTMLButtonElement>('[data-lab-action="jump-rules"]')!;
  expect(edit.textContent).toBe("Edit indicators & rules");
  await act(async () => edit.click());
  const left = host.querySelector('.lab-rules [aria-label="Entry 1 left"]')!;
  expect(left.querySelector('button[aria-label="Indicator"]')).not.toBeNull();
  await pickIndicator(left, "ema");
  await run(host);
  const spec = JSON.parse(new URL(f.calls.find((c) => c.url.includes("/lab/run"))!.url, "http://x").searchParams.get("spec")!);
  expect(spec.entry.all[0].lhs.s).toBe("ema");
  expect(spec.origin).toEqual({book:"ststw", method:"rsi2_4", template_version:1});
});

it("exposes foreign source choices only for an advertised engine and sends the edited rule", async () => {
  catalogueAnswer={...catalogue7Fixture,max_foreign:4} as typeof catalogueFixture;
  const {host,f}=await go("#/indicators?tk=NIFTY%2050");
  await act(async()=>host.querySelector<HTMLButtonElement>("[data-lab-param-more]")!.click()); // Symbol sits behind More
  const source=host.querySelector<HTMLSelectElement>('select[aria-label="Entry 1 check symbol"]');
  expect(source).not.toBeNull();
  const other=[...source!.options].find(o=>o.value&&o.value!=="NIFTY 50")!.value;
  await act(async()=>{source!.value=other;source!.dispatchEvent(new Event("change",{bubbles:true}));});await run(host);
  const request=f.calls.find(c=>c.url.includes("/lab/run"));expect(request).toBeDefined();
  expect(new URL(request!.url,"http://x").searchParams.get("spec")).toContain(`"symbol":"${other}"`);
});

it("keeps foreign controls absent on older engines and blocks a saved invalid foreign spec before HTTP", async () => {
  const old=await go("#/indicators");expect(old.host.querySelector('[aria-label$="· Read from"]')).toBeNull();await unmount();clearLabCache();
  catalogueAnswer={...catalogue7Fixture,max_foreign:4} as typeof catalogueFixture;
  if(templateFixture.status!=="available")throw new Error("fixture must be available");
  templateAnswer={...templateFixture,template:{...templateFixture.template,spec:{...templateFixture.template.spec,entry:{all:[{lhs:{s:"close",symbol:"UNKNOWN"},op:">",rhs:{value:1}}]}}}} as typeof templateFixture;
  const {host,f}=await go("#/indicators?customize=rsi2_4&tk=NIFTY%2050");
  expect(host.textContent).toContain("UNKNOWN is not listed");expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  await run(host);expect(f.calls.filter(c=>c.url.includes("/lab/run"))).toHaveLength(0);
});

it("runs a shared-money portfolio while leaving the chart instrument as reference context",async()=>{
 catalogueAnswer=portfolioCatalogueFixture;runAnswer=()=>jsonResponse(portfolioResultFixture);
 const {host,f}=await go("#/indicators?tk=NIFTY%2050");
 const choose=async(label:string,value:string)=>act(async()=>{const el=host.querySelector<HTMLSelectElement>(`[aria-label="${label}"]`)!;el.value=value;el.dispatchEvent(new Event("change",{bubbles:true}));});
 await choose("Strategy mode","signals");await choose("Portfolio universe","nifty50");await exitRow(host);
 const money=[...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find(b=>b.textContent==="Cash amounts")!;
 expect(money.disabled).toBe(false);await act(async()=>money.click());
 expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(false);await run(host);
 const call=f.calls.find(c=>c.url.includes("/lab/run"))!;const q=new URL(call.url,"http://x").searchParams;expect(q.has("ticker")).toBe(false);expect(q.has("cost")).toBe(false);
 expect(host.querySelector('[data-lab-portfolio-result]')).not.toBeNull();expect(host.querySelector('[data-lab-result]')).toBeNull();expect(host.textContent).toContain("ACTUAL");
});
it("an old portfolio run cannot land after its universe changes",async()=>{
 catalogueAnswer=portfolioCatalogueFixture;let release!:(r:Response)=>void;runAnswer=()=>new Promise<Response>(resolve=>{release=resolve;});
 const {host}=await go("#/indicators?tk=NIFTY%2050");const choose=async(label:string,value:string)=>act(async()=>{const el=host.querySelector<HTMLSelectElement>(`[aria-label="${label}"]`)!;el.value=value;el.dispatchEvent(new Event("change",{bubbles:true}));});
 await choose("Strategy mode","signals");await choose("Portfolio universe","nifty50");await exitRow(host);await act(async()=>[...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find(b=>b.textContent==="Cash amounts")!.click());await run(host);
 await choose("Portfolio universe","list");await act(async()=>release(jsonResponse(portfolioResultFixture)));await flush();
 expect(host.querySelector('[data-lab-portfolio-result]')).toBeNull();expect(host.querySelector('[data-lab-no-result]')).not.toBeNull();
});
