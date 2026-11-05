import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import { LabModeContext, POPULAR } from "../src/features/lab/simple";
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
/** There is no hidden default exit: a test needs an exit picked in the Exit card (N candle, 10 candles). */
const pickExit = (host: HTMLElement) => click(host.querySelector('[data-lab-exit="ncandle"]'));
const button = (host: HTMLElement, text: string) => [...host.querySelectorAll("button")].find((b) => b.textContent?.trim() === text) ?? null;
/** Pick another indicator for the first buy rule, as a user does in the Entry card. */
const pickIndicator = async (host: HTMLElement, id: string) => {
  const row = () => host.querySelector<HTMLElement>('[data-lab-list="entry.all"] [data-lab-param]')!;
  await click(row().querySelector('button[aria-label="Indicator"]'));
  await act(async () => { row().querySelector(`li[data-id="${id}"]`)!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })); }); await flush();
};
const typeInto = async (input: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
});

it("opens in the Simple view: Simple / Advanced only trims Settings (no AmiBroker settings); the rules and tabs are the full ones", async () => {
  const host = await open();
  expect(host.querySelector("[data-lab-settings-toggle]")).toBeNull(); // no Show / Hide settings button: Settings is always open
  expect(host.querySelector<HTMLElement>("[data-lab-settings-body]")!.hidden).toBe(false);
  expect(host.querySelector('.lab-settings-aside [data-lab-mode="simple"]')).not.toBeNull(); // the switch is in the Settings heading
  expect(host.querySelector('.lab-tabs-row [data-lab-mode]')).toBeNull(); // not beside the workspace tabs
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => t.textContent)).toEqual(["Chart", "Strategy & backtest", "Customize", "Saved strategies"]);
  const rules = host.querySelector(".lab-rules")!;
  expect(rules.querySelector(".box-h")!.firstChild!.textContent).toBe("Strategy rules"); // no "values from completed bars only" note
  expect([...rules.querySelectorAll("[data-lab-card] .lab-rule-section-h h3")].slice(0, 3).map((h) => h.textContent)).toEqual(["Entry", "Exit", "Filters"]); // each rule section has a descriptive semantic heading
  expect(rules.querySelector("[data-lab-settings-tabs]")).toBeNull();
  expect(rules.querySelector("button[data-lab-add-cond]")!.textContent).toBe("+ Add condition"); // the full builder: + Add condition and + Add group
  expect(rules.querySelector("select[data-lab-add]")).not.toBeNull();
  expect(rules.querySelector('[data-lab-idea-basics] [data-lab-action="run"]')!.textContent).toBe("Run backtest");
  // Positions and the test period sit over the rule cards, as in Advanced
  expect(rules.querySelector('[data-lab-idea-basics] [data-lab-exec="positions"]')).not.toBeNull();
  expect(rules.querySelector('[data-lab-idea-basics] input[aria-label="From"]')).not.toBeNull();
  // step 2 Settings: the AFLs' Backtest parameters, no AmiBroker settings and no % / cash toggle (money follows the instrument)
  const settings = host.querySelectorAll(".lab-rules")[1];
  expect([...settings.querySelectorAll("[data-lab-basics] .lab-sgroup > h4")].map((h) => h.textContent)).toEqual(["Backtest parameters"]);
  expect(settings.querySelector('input[aria-label="Brokerage (bps / side)"]')).not.toBeNull();
  expect(settings.querySelector('input[aria-label="Slippage (bps / side)"]')).toBeNull(); // the AFL panel has BROKERAGE only
  expect(settings.querySelector("[data-lab-more-settings]")).toBeNull();
  expect(settings.querySelector('[aria-label="Results in"]')).toBeNull();
  expect(host.querySelector('input[aria-label="Tick size"]')).toBeNull();
});

it("a new strategy trades at the signal day's close; Buy and sell at is the Trades tab's Scenario, not a Settings basic", async () => {
  const host = await open();
  const settings = host.querySelectorAll(".lab-rules")[1] as HTMLElement;
  expect(settings.querySelector('[aria-label="Buy and sell at"]')).toBeNull();
  await pickExit(host);
  await click(host.querySelector('[data-lab-action="run"]'));
  const run = (fetch as unknown as ReturnType<typeof routes>).calls.filter((c) => c.url.includes("/lab/run")).at(-1)!;
  const spec = JSON.parse(new URL(run.url, "http://x").searchParams.get("spec")!);
  expect(spec.execution).toMatchObject({ fill: "close" });
  expect(spec.execution.exit_fill).toBeUndefined();
});

it("a new strategy shows one indicator rule as a parameter list; + Add condition adds another", async () => {
  const host = await open();
  const lists = () => host.querySelectorAll(".lab-list");
  expect(host.querySelector("[data-lab-starter], [data-lab-example], [data-lab-pick]")).toBeNull(); // no suggestions: + Add condition starts the rule
  const row = () => lists()[0].querySelector<HTMLElement>("[data-lab-param]")!;
  expect(lists()[0].querySelectorAll("[data-lab-param]")).toHaveLength(1);
  expect(row().querySelector('button[aria-label="Indicator"]')!.textContent).toContain("RSI");
  expect([...row().querySelectorAll<HTMLInputElement>("input[type=number]")].map((i) => i.value)).toEqual(["14", "30"]); // period and threshold; bars ago is behind More
  await typeInto(row().querySelector<HTMLInputElement>('input[aria-label="Entry 1 check Period"]')!, "2");
  expect(host.querySelector("[data-lab-words]")!.textContent).toContain("RSI(2)"); // the period changed in place
  expect(lists()[0].querySelector(".lab-list-help, [data-lab-count]")).toBeNull(); // no description, no count row
  await click(lists()[0].closest("[data-lab-card]")!.querySelector("button[data-lab-add-cond]"));
  expect(lists()[0].querySelectorAll("[data-lab-param]")).toHaveLength(2);
});

it("an oscillator is read against a threshold only; a price-chart line keeps Compare with; no level buttons", async () => {
  const host = await open();
  const row = () => host.querySelector<HTMLElement>(".lab-list [data-lab-param]")!;
  const compare = () => row().querySelector('select[aria-label="Entry 1 compare with type"]');
  expect(compare()).toBeNull();
  expect(row().querySelector('input[aria-label="Entry 1 compare with threshold"]')).not.toBeNull();
  expect(host.querySelector("[data-lab-levels]")).toBeNull();
  await pickIndicator(host, "ema"); // a moving average is compared with the close or another line
  expect(compare()).not.toBeNull();
});

it("Simple shows the same Results tabs as Advanced, the trades first", async () => {
  const host = await open();
  await pickExit(host);
  await click(host.querySelector('[data-lab-action="run"]'));
  expect(host.querySelector("[data-lab-results-tabs]")).not.toBeNull();
  expect(host.querySelector("[data-lab-more]")).toBeNull();
  expect([...host.querySelectorAll("[data-lab-results-tabs] [role=tab]")].map((t) => t.textContent)[0]).toBe("Trades");
});

it("Advanced restores every control and is remembered", async () => {
  const host = await open();
  await click(button(host, "Advanced"));
  expect(localStorage.getItem("lab.mode")).toBe("advanced");
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => t.textContent)).toContain("Customize");
  expect(host.querySelector("[data-lab-open-optimize]")).not.toBeNull(); // Optimize beside Run backtest
  expect(host.querySelector("[data-lab-settings-tabs]")).not.toBeNull();
  expect(host.querySelector("[data-lab-node-menu]")).toBeNull(); // a plain rule is one parameter list
  await click(host.querySelector("[data-lab-param-more]"));
  expect(host.querySelector('input[aria-label="Entry 1 check bars ago"]')).not.toBeNull(); // More: bars ago, symbol, calculation
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

it("a new strategy: the RSI rule and the ATR exit, filters off; it runs as shown", async () => {
  const host = await open();
  const words = () => host.querySelector("[data-lab-words]")!.textContent;
  expect(words()).toContain("Buy when RSI(14) crosses above 30.");
  expect(words()).not.toContain("200-day EMA"); // no filter by default
  expect(words()).toContain("ATR (5, 2) (5, 4)");
  expect(host.querySelector("[data-lab-default-exit]")).toBeNull(); // no hidden default exit: the ATR is in the Exit card
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(false);
  await click(host.querySelector('[data-lab-action="run"]'));
  const run = (fetch as unknown as ReturnType<typeof routes>).calls.filter((c) => c.url.includes("/lab/run")).at(-1)!;
  const spec = JSON.parse(new URL(run.url, "http://x").searchParams.get("spec")!) as Spec;
  expect(spec.exit.dual_atr).toEqual({ initial: { period: 5, mult: 2, price: "hl2" }, favour: { period: 5, mult: 4, price: "hl2" } });
  expect(spec.entry.all.some((n) => (n as { filter?: boolean }).filter === true)).toBe(false);
});

it("one Compare with by default; More adds lines, each Required or Any one", async () => {
  const host = await open();
  const row = () => host.querySelector<HTMLElement>('[data-lab-list="entry.all"] [data-lab-param]')!;
  const words = () => host.querySelector("[data-lab-words-buy]")!.textContent;
  const kind = row().querySelector<HTMLSelectElement>('select[aria-label="Entry 1 check type"]')!;
  await act(async () => { kind.value = "price"; kind.dispatchEvent(new Event("change", { bubbles: true })); }); await flush(); // close, compared with a line
  expect(row().querySelector("[data-lab-add-line]")).toBeNull(); // hidden until More
  await click(row().querySelector("[data-lab-param-more]"));
  await click(row().querySelector("[data-lab-add-line]"));
  expect(row().querySelectorAll(".lab-param-line")).toHaveLength(2);
  expect(row().querySelector("[data-lab-must-rule]")!.textContent).toBe("Every line is required.");
  const set = (line: number, v: "Required" | "Any one") => click([...row().querySelectorAll<HTMLButtonElement>(`[aria-label="Entry 1 line ${line} is"] button`)].find((b) => b.textContent === v)!);
  await set(2, "Any one"); // required lines first, then the any-one lines under their own heading
  expect(row().querySelector("[data-lab-either-head]")!.textContent).toBe("Plus any one of these:");
  await set(1, "Any one");
  expect(row().querySelector("[data-lab-must-rule]")!.textContent).toBe("Any one line is enough.");
  expect(row().querySelector("[data-lab-either-head]")!.textContent).toBe("Any one of these:");
  expect(words()).toMatch(/ or /);
  await click(row().querySelector("[data-lab-add-line]")); // a new line is Required: it, plus any one of the others
  expect(row().querySelector("[data-lab-must-rule]")!.textContent).toBe("The required line, plus any one of the others.");
  expect(words()).toMatch(/ and either .* or /);
  await click(row().querySelector('button[aria-label="Entry 1 remove line 3"]'));
  await click(row().querySelector('button[aria-label="Entry 1 remove line 2"]'));
  expect(row().querySelectorAll(".lab-param-line")).toHaveLength(0); // back to one Compare with
});

it("switching Check from an indicator to a price compares the price with an EMA, not the indicator's old threshold", async () => {
  const host = await open();
  const row = () => host.querySelector<HTMLElement>('[data-lab-list="entry.all"] [data-lab-param]')!;
  const kind = row().querySelector<HTMLSelectElement>('select[aria-label="Entry 1 check type"]')!;
  await act(async () => { kind.value = "price"; kind.dispatchEvent(new Event("change", { bubbles: true })); }); await flush();
  expect(host.querySelector("[data-lab-words-buy]")!.textContent).toContain("Buy when the close crosses above the 20-day EMA.");
  expect(row().querySelector<HTMLSelectElement>('select[aria-label="Entry 1 compare with type"]')!.value).toBe("indicator");
});
