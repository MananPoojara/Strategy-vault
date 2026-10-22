import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { IndicatorsPage } from "../src/pages/IndicatorsPage";
import { clearLabCache } from "../src/features/lab/client";
import { EXAMPLES, POPULAR, withExample } from "../src/features/lab/simple";
import { catalogue7Fixture, fetchRouter, flush, glInstrumentsFixture, jsonResponse, mount, publishedFixture, runFixture, seriesFixture, templatesFixture, tickersFixture, unmount } from "./fixtures/lab";
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

it("opens in the Simple view: no power-user controls, examples, a one-line settings summary and a plain run button", async () => {
  const host = await open();
  expect(host.querySelector('[data-lab-mode="simple"]')).not.toBeNull();
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => t.textContent)).toEqual(["Strategy & backtest", "Saved strategies"]);
  const rules = host.querySelector(".lab-rules")!;
  expect(rules.querySelector(".box-h")!.textContent).toContain("Your idea");
  expect(rules.textContent).toContain("Buy when all of these are true");
  expect(rules.textContent).toContain("Sell when any of these is true");
  for (const hidden of ["[data-lab-caps]", "[data-lab-node-menu]", "[data-lab-foreign]", "[data-lab-settings-tabs]"]) expect(rules.querySelector(hidden), hidden).toBeNull();
  expect(rules.querySelector("button[data-lab-add]")!.textContent).toBe("+ Add condition");
  expect(rules.querySelector('[data-lab-action="run"]')!.textContent).toBe("▶ Test this idea");
  // step 3 Settings: one summary line, already open on only the period, costs, long/short and starting equity
  const settings = host.querySelectorAll(".lab-rules")[1];
  expect(settings.querySelector("[data-lab-simple-settings]")!.textContent).toContain("All data");
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
  await click(host.querySelector('[data-lab-action="run"]'));
  const run = (fetch as unknown as ReturnType<typeof routes>).calls.filter((c) => c.url.includes("/lab/run")).at(-1)!;
  const spec = JSON.parse(new URL(run.url, "http://x").searchParams.get("spec")!);
  expect(spec.execution).toMatchObject({ fill: "close" });
  expect(spec.execution.exit_fill).toBeUndefined();
});

it("an example fills the rules; adding a condition is one button", async () => {
  const host = await open();
  const lists = () => host.querySelectorAll(".lab-list");
  await click(host.querySelector('[data-lab-example="rsi_oversold"]'));
  expect(lists()[0].querySelectorAll("[data-lab-node]")).toHaveLength(1);
  expect(lists()[0].querySelector<HTMLSelectElement>('select[aria-label="Indicator"]')?.value).toBe("rsi");
  await click(lists()[0].querySelector("button[data-lab-add]"));
  expect(lists()[0].querySelectorAll("[data-lab-node]")).toHaveLength(2);
});

it("a rule on an oscillator offers its usual levels; a click compares against that number", async () => {
  const host = await open();
  const row = () => host.querySelector(".lab-list [data-lab-node]")!;
  await click(host.querySelector('[data-lab-example="ma_cross"]'));
  expect(row().querySelector("[data-lab-levels]")).toBeNull(); // a moving average has no levels
  await click(host.querySelector('[data-lab-example="rsi_oversold"]'));
  expect([...row().querySelectorAll("[data-lab-levels] button")].map((b) => b.textContent)).toEqual(["70", "30"]);
  await click(row().querySelector("[data-lab-levels] button"));
  expect([...row().querySelectorAll<HTMLInputElement>("input")].map((i) => i.value)).toContain("70");
});

it("results lead with the idea against buy & hold; More detail opens the full tabs", async () => {
  const host = await open();
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
  await click(button(host, "Advanced"));
  expect(localStorage.getItem("lab.mode")).toBe("advanced");
  expect([...host.querySelectorAll(".lab-tabs [role=tab]")].map((t) => t.textContent)).toContain("Optimization");
  expect(host.querySelector("[data-lab-settings-tabs]")).not.toBeNull();
  expect(host.querySelector("[data-lab-node-menu]")).toBeNull(); // behind More
  await click(host.querySelector(".lab-cond .lab-more-btn"));
  expect(host.querySelector("[data-lab-node-menu]")).not.toBeNull();
  expect(host.querySelector(".lab-rules .box-h")!.textContent).toContain("Strategy rules");
});

it("the Library opens on the popular indicators, with every category one click away", async () => {
  const host = await open();
  const popular = POPULAR.filter((id) => catalogue7Fixture.indicators.some((i) => i.id === id));
  expect([...host.querySelectorAll("[data-lab-library] [data-lab-item]")].map((b) => (b as HTMLElement).dataset.labItem)).toEqual(popular);
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
