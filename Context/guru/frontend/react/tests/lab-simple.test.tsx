import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import { EXAMPLES, LabModeContext, POPULAR, withExample } from "../src/features/lab/simple";
import { catalogue7Fixture, fetchRouter, flush, glInstrumentsFixture, jsonResponse, mount, publishedFixture, runFixture, seriesFixture, templatesFixture, tickersFixture, unmount } from "./fixtures/lab";
import { LibraryRail } from "../src/features/lab/LibraryRail";
import type { Spec } from "../src/features/lab/types";

const routes = () => fetchRouter([
  ["/api/app/me", () => jsonResponse({ user: { id: "u1", name: "N", email: "n@x", role: "researcher" }, settings: { theme: "aqua" } })],
  ["/api/app/lab/strategies/published", () => jsonResponse(publishedFixture)],
  ["/api/app/lab/strategies", () => jsonResponse([])],
  ["/api/app/lab/indicators", () => jsonResponse([])],
  [/\/api\/(in|us|gl)\/lab\/catalogue/, () => jsonResponse(catalogue7Fixture)],
  ["/api/in/tickers", () => jsonResponse(tickersFixture)],
  ["/api/gl/lab/instruments", () => jsonResponse(glInstrumentsFixture)],
  [/\/lab\/series/, (url) => { const id = new URL(url, "http://x").searchParams.get("id")!; return jsonResponse(seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] })); }],
  [/\/lab\/templates/, () => jsonResponse(templatesFixture)],
  [/\/lab\/run/, () => jsonResponse(runFixture)],
]);
beforeEach(() => { localStorage.removeItem("lab.mode"); clearLabCache(); }); // the real default: no stored choice
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); history.replaceState(null, "", "/"); localStorage.setItem("lab.mode", "advanced"); });
const open = async () => {
  history.replaceState(null, "", "/#/indicators?tk=NIFTY%2050");
  vi.stubGlobal("fetch", routes());
  const host = await mount(<IndicatorsPage />);
  await flush(); await flush();
  return host;
};
const click = async (el: Element | null) => { await act(async () => (el as HTMLElement).click()); await flush(); };
const button = (host: HTMLElement, text: string) => [...host.querySelectorAll("button")].find((b) => b.textContent?.trim() === text) ?? null;
/** A new strategy starts with no buy rule: take the RSI suggestion. */
const pickRsi = (host: HTMLElement) => click(host.querySelector('[data-lab-pick="rsi"]'));
const typeInto = async (input: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
});

it("opens in the Simple view: no power-user controls, examples, a one-line settings summary and a plain run button", async () => {
  const host = await open();
  expect(host.querySelector('[data-lab-mode="simple"]')).not.toBeNull();
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => t.textContent)).toEqual(["Chart", "Strategy & backtest", "Saved strategies"]);
  const rules = host.querySelector(".lab-rules")!;
  expect(rules.querySelector(".box-h")!.textContent).toContain("Your idea");
  expect(rules.textContent).toContain("Buy when all of these are true");
  expect(rules.textContent).toContain("Sell when any of these is true");
  for (const hidden of ["[data-lab-caps]", "[data-lab-node-menu]", "[data-lab-foreign]", "[data-lab-settings-tabs]"]) expect(rules.querySelector(hidden), hidden).toBeNull();
  expect(rules.querySelector("button[data-lab-add]")!.textContent).toBe("+ Add condition");
  expect(rules.querySelector('[data-lab-action="run"]')!.textContent).toBe("▶ Test this idea");
  // step 3 Settings: one summary line, already open on only the period, costs, long/short and starting equity
  const settings = host.querySelectorAll(".lab-rules")[1];
  expect(settings.querySelector("[data-lab-simple-settings]")!.textContent).toMatch(/01-01-2000 → \d\d-\d\d-\d{4} \(latest data\)/); // tests run from 2000 to the latest data unless changed
  expect(settings.querySelector("[data-lab-simple-settings]")!.textContent).toContain("long only");
  expect(button(settings as HTMLElement, "Done")).not.toBeNull();
  expect(settings.querySelector('input[aria-label="From"]')).not.toBeNull();
  expect(settings.querySelector('[aria-label="Results in"]')!.textContent).toBe("% returnsCash amounts");
  expect(host.querySelector('input[aria-label="Tick size"]')).toBeNull();
});

it("a new strategy trades at the signal day's close; Simple can switch to the next open", async () => {
  const host = await open();
  const settings = host.querySelectorAll(".lab-rules")[1] as HTMLElement;
  const summary = () => settings.querySelector("[data-lab-simple-settings]")!.textContent;
  expect(summary()).toContain("trade at the signal day's close");
  await click(button(settings, "Next day's open"));
  expect(summary()).toContain("trade at the next day's open");
  await click(button(settings, "Signal day's close"));
  await pickRsi(host);
  await click(host.querySelector('[data-lab-action="run"]'));
  const run = (fetch as unknown as ReturnType<typeof routes>).calls.filter((c) => c.url.includes("/lab/run")).at(-1)!;
  const spec = JSON.parse(new URL(run.url, "http://x").searchParams.get("spec")!);
  expect(spec.execution).toMatchObject({ fill: "close" });
  expect(spec.execution.exit_fill).toBeUndefined();
});

it("an example fills the rules as sentences; adding a condition opens the new row's editor", async () => {
  const host = await open();
  const lists = () => host.querySelectorAll(".lab-list");
  await click(host.querySelector('[data-lab-example="rsi_oversold"]'));
  const row = lists()[0].querySelector("[data-lab-sentence]")!;
  expect(lists()[0].querySelectorAll("[data-lab-sentence]")).toHaveLength(1);
  expect(row.textContent).toContain("RSI(");
  expect([...row.querySelectorAll<HTMLInputElement>("input")].map((i) => i.value)).toEqual(["14", "30"]);
  await typeInto(row.querySelector<HTMLInputElement>('input[aria-label="Entry 1: Relative Strength Index Length"]')!, "2");
  expect(lists()[0].querySelector("[data-lab-sentence]")!.textContent).toContain("is below"); // the number changed in place
  await click(lists()[0].querySelector("button[data-lab-add]"));
  expect(lists()[0].querySelectorAll("[data-lab-sentence]")).toHaveLength(1);
  expect(lists()[0].querySelector("[data-lab-sentence-edit] [data-lab-node]")).not.toBeNull();
  await click(button(lists()[0] as HTMLElement, "Done"));
  expect(lists()[0].querySelectorAll("[data-lab-sentence]")).toHaveLength(2);
});

it("a rule on an oscillator offers its usual levels; a click compares against that number", async () => {
  const host = await open();
  const row = () => host.querySelector(".lab-list [data-lab-node]")!;
  const edit = async () => { if (!host.querySelector(".lab-list [data-lab-sentence-edit]")) await click(button(host.querySelector(".lab-list [data-lab-sentence]") as HTMLElement, "Edit")); };
  await click(host.querySelector('[data-lab-example="ma_cross"]'));
  await edit();
  expect(row().querySelector("[data-lab-levels]")).toBeNull(); // a moving average has no levels
  await click(host.querySelector('[data-lab-example="rsi_oversold"]'));
  await edit();
  expect([...row().querySelectorAll("[data-lab-levels] button")].map((b) => b.textContent)).toEqual(["70", "30"]);
  await click(row().querySelector("[data-lab-levels] button"));
  expect([...row().querySelectorAll<HTMLInputElement>("input")].map((i) => i.value)).toContain("70");
});

it("results lead with the idea against buy & hold; More detail opens the full tabs", async () => {
  const host = await open();
  await pickRsi(host);
  await click(host.querySelector('[data-lab-action="run"]'));
  const summary = host.querySelector("[data-lab-simple-result]")!;
  expect(summary.textContent).toContain("Your idea");
  expect(summary.textContent).toContain("Buy & hold");
  expect([...summary.querySelectorAll("tbody th")].map((t) => t.textContent)).toEqual(["Total return", "Per year", "Worst fall"]);
  expect(host.querySelector("[data-lab-result]")).not.toBeNull(); // the equity chart
  expect(host.querySelector("[data-lab-results-tabs]")).toBeNull();
  await click(host.querySelector("[data-lab-more]"));
  expect(host.querySelector("[data-lab-results-tabs]")).not.toBeNull();
});

it("Advanced restores every control and is remembered", async () => {
  const host = await open();
  await pickRsi(host);
  await click(button(host, "Advanced"));
  expect(localStorage.getItem("lab.mode")).toBe("advanced");
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => t.textContent)).toContain("Optimization");
  expect(host.querySelector("[data-lab-settings-tabs]")).not.toBeNull();
  expect(host.querySelector("[data-lab-node-menu]")).toBeNull(); // behind More
  await click(host.querySelector(".lab-cond .lab-more-btn"));
  expect(host.querySelector("[data-lab-node-menu]")).not.toBeNull();
  expect(host.querySelector(".lab-rules .box-h")!.textContent).toContain("Strategy rules");
});

it("Simple chart opens the same searchable popup with all indicators and no library mode toggle", async () => {
  const host = await open();
  await click([...host.querySelectorAll(".lab-chart-actions button")].find((b) => b.textContent === "Indicators") ?? null);
  const popup = host.querySelector('[role="dialog"][aria-label="Indicators"]')!;
  expect(popup).not.toBeNull();
  expect(popup.querySelectorAll("[data-lab-library] [data-lab-item]")).toHaveLength(catalogue7Fixture.indicators.length);
  expect(popup.querySelector("[data-lab-show-all]")).toBeNull();
  const search = popup.querySelector<HTMLInputElement>('[aria-label="Search indicators"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(search, "ta.rsi");
    search.dispatchEvent(new Event("input", { bubbles:true }));
  });
  expect([...popup.querySelectorAll<HTMLElement>("[data-lab-item]")].map(row => row.dataset.labItem)).toEqual(["rsi"]);
});

it("the standalone Simple library preserves its popular shortlist and show-all control", async () => {
  const host = await mount(<LabModeContext.Provider value="simple"><LibraryRail catalogue={catalogue7Fixture} market="in" hasVolume tf="D" onAdd={() => {}} onRead={() => {}} /></LabModeContext.Provider>);
  const popular = POPULAR.filter((id) => catalogue7Fixture.indicators.some((i) => i.id === id));
  expect([...host.querySelectorAll<HTMLElement>("[data-lab-library] [data-lab-item]")].map(button => button.dataset.labItem)).toEqual(popular);
  await click(host.querySelector("[data-lab-show-all]"));
  expect(host.querySelectorAll("[data-lab-library] [data-lab-item]")).toHaveLength(catalogue7Fixture.indicators.length);
});

it("every example is a complete entry and exit", () => {
  const base = { v: 1, entry: { all: [] }, exit: { any: [] } } as unknown as Spec;
  for (const e of EXAMPLES) {
    const s = withExample(base, e.id);
    expect(s.entry.all.length && s.exit.any.length, e.id).toBeTruthy();
  }
});

it("a new strategy starts empty: suggestions, the default exit in words, and the test uses it", async () => {
  const host = await open();
  const words = () => host.querySelector("[data-lab-words]")!.textContent;
  expect(host.querySelector(".lab-list [data-lab-sentence]")).toBeNull();
  expect(host.querySelector("[data-lab-starter]")).not.toBeNull();
  expect(words()).toContain("no buy rule yet");
  expect(words()).toContain("Sell at the 1st up close after entry, or at the 4th down close.");
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  expect(host.querySelector('[data-lab-list="entry.all.filters"]')).toBeNull(); // no filters in a plain entry and exit
  await pickRsi(host); // TradingView's RSI Strategy: its buy rule and its own exit
  expect(words()).toContain("Buy when RSI(14) crosses above 30.");
  expect(words()).toContain("Sell when RSI(14) crosses below 70.");
  expect(host.querySelector("[data-lab-starter]")).toBeNull();
  const sell = () => host.querySelector('[data-lab-list="exit.any"]')!;
  await click([...sell().querySelectorAll<HTMLButtonElement>("[data-lab-sentence] button")].find((b) => b.textContent === "×")!);
  expect(sell().querySelector("[data-lab-own-exit]")!.textContent).toBe("RSI 14's own exit"); // offered back on the default-exit card
  await typeInto(host.querySelector<HTMLInputElement>('[data-lab-default-exit] input[aria-label="Default exit: which up close"]')!, "2");
  expect(words()).toContain("at the 2nd up close after entry");
  expect(JSON.parse(localStorage.getItem("lab.defaultExit")!)).toEqual({ up: 2, down: 4 });
  await click(host.querySelector('[data-lab-action="run"]'));
  const run = (fetch as unknown as ReturnType<typeof routes>).calls.filter((c) => c.url.includes("/lab/run")).at(-1)!;
  const spec = JSON.parse(new URL(run.url, "http://x").searchParams.get("spec")!) as Spec;
  expect(spec.exit.any.map((n) => (n as { rhs: { value: number } }).rhs.value)).toEqual([2, 4]);
  expect(host.querySelector("[data-lab-default-exit-used]")!.textContent).toContain("sell at the 2nd up close after entry");
  localStorage.removeItem("lab.defaultExit");
});
