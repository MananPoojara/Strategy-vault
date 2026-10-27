import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { defaultSpec, addCond } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, catalogue7Fixture, mount, pickIndicator, templateFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const settings = { from: "", to: "", fees: "", slippage: "" };
const setup = async (spec: Spec, extra: Partial<Parameters<typeof RuleBuilder>[0]> = {}) => {
  const onChange = vi.fn(), onRun = vi.fn(), onSettings = vi.fn();
  const host = await mount(<RuleBuilder spec={spec} catalogue={{ ...cat, timeframes: ["D", "W", "M", "Q"] }} onChange={onChange}
    settings={settings} onSettings={onSettings} problems={[]} running={false} onRun={onRun} {...extra} />);
  return { host, onChange, onRun, onSettings, last: () => onChange.mock.calls.at(-1)![0] as Spec };
};
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });

it("shows both sides, catalogue operators incl. 'is at most' and catalogue timeframes", async () => {
  const { host } = await setup(defaultSpec());
  expect(host.textContent).toContain("Entry · all of these");
  expect(host.textContent).toContain("Exit · any of these");
  expect([...host.querySelectorAll("option")].map((o) => o.textContent)).toEqual(expect.arrayContaining(["is at most", "crosses above"]));
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-timing]")!.click());
  expect([...host.querySelectorAll("option")].some((o) => o.value === "Q")).toBe(true);
});

it("bars held only on the exit side; listed indicators are never offered", async () => {
  const { host } = await setup(defaultSpec());
  const [entry, exit] = host.querySelectorAll<HTMLElement>(".lab-side");
  expect(entry.textContent).not.toContain("Bars held");
  expect(exit.textContent).toContain("Bars held");
  expect(host.querySelector('option[value="or_range"]')).toBeNull();
});

it("editing keeps origin and unknown keys", async () => {
  const spec = { ...(templateFixture.status === "available" ? templateFixture.template.spec : defaultSpec()), future_rule: 1 } as Spec;
  const { host, last } = await setup(spec);
  await choose(host.querySelector<HTMLSelectElement>(".lab-cond select[aria-label='Operator']")!, "<=");
  expect(last().entry.all[0].op).toBe("<=");
  expect(last().origin).toEqual(spec.origin);
  expect(last().future_rule).toBe(1);
});

it("unknown operand shapes are kept and shown read-only", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { fancy: { k: 1 } } as never }] } } as Spec;
  const { host } = await setup(spec);
  expect(host.querySelector(".lab-kept")?.textContent).toContain('{"fancy":{"k":1}}');
});

it("add condition stops at the catalogue limit; problems block the run", async () => {
  let spec = defaultSpec();
  for (let i = 0; i < 10; i++) spec = addCond(spec, "entry", spec.entry.all[0]);
  const { host } = await setup(spec, { problems: ["Entry 1: a condition cannot compare two numbers; one side must be a series."] });
  expect([...host.querySelectorAll<HTMLButtonElement>("button")].filter((b) => b.textContent === "+ Add condition").every((b) => b.disabled)).toBe(true);
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  expect(host.textContent).toContain("one side must be a series");
});

it("orders: a limit hides the entry fill and removes it; ATR stops; short direction", async () => {
  const { host, last } = await setup({ ...defaultSpec(), execution: { fill: "close" } });
  const row = (label: string) => [...host.querySelectorAll<HTMLElement>(".lab-order")].find((r) => r.textContent!.startsWith(label))!;
  await act(async () => [...row("Limit entry").querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "%")!.click());
  expect(last().entry.limit).toBeDefined();
  // fill also set the exit fill, so the model keeps it as exit_fill (Task 4 review fix).
  expect(last().execution).toEqual({ exit_fill: "close" });
  await act(async () => [...row("Stop-loss").querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "ATR")!.click());
  expect(last().exit.stop_loss).toEqual({ atr: 2, atr_length: 14 });
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "Short")!.click());
  expect(last().direction).toBe("short");
});

it("the fees box edits cost_bps when the spec carries it", async () => {
  const { host, last, onSettings } = await setup({ ...defaultSpec(), execution: { cost_bps: 10 } });
  const fees = host.querySelector<HTMLInputElement>('input[aria-label="Fees (bps / side)"]')!;
  expect(fees.value).toBe("10");
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(fees, "12");
    fees.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(last().execution?.cost_bps).toBe(12);
  expect(onSettings).not.toHaveBeenCalledWith(expect.objectContaining({ fees: "12" }));
});

const typeIn = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});

it("operand edits: D deletes tf, 0 deletes offset, a new indicator drops params, price keeps tf", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close", tf: "W", offset: 2 }, op: ">", rhs: { s: "macd", p: { fast: 5 }, out: "signal", tf: "W" }, note: "x" }] } } as Spec;
  const { host, last } = await setup(spec);
  expect(host.textContent).toContain("Higher-timeframe values are used only after that period closes (unless a timing option says otherwise).");
  const lhs = host.querySelector<HTMLElement>('[aria-label="Entry 1 left"]')!, rhs = host.querySelector<HTMLElement>('[aria-label="Entry 1 right"]')!;
  await act(async () => lhs.querySelector<HTMLButtonElement>("[data-lab-timing]")!.click());
  await choose(lhs.querySelector<HTMLSelectElement>('select[aria-label="Timeframe"]')!, "D");
  expect(last().entry.all[0].lhs).toEqual({ s: "close", offset: 2 });
  expect(last().entry.all[0].note).toBe("x");
  await typeIn(lhs.querySelector<HTMLInputElement>('input[aria-label="periods ago"]')!, "0");
  expect(last().entry.all[0].lhs).toEqual({ s: "close", tf: "W" });
  expect(rhs.querySelector<HTMLSelectElement>('select[aria-label="Output"]')!.value).toBe("signal");
  await pickIndicator(rhs, "rsi");
  expect(last().entry.all[0].rhs).toEqual({ s: "rsi", tf: "W" });
  await choose(rhs.querySelector<HTMLSelectElement>('select[aria-label="Operand type"]')!, "price");
  expect(last().entry.all[0].rhs).toEqual({ s: "close", tf: "W" });
});

it("orders Off removes the key; exec line follows fill; remove condition", async () => {
  const { host, last } = await setup({ ...defaultSpec(), exit: { any: defaultSpec().exit.any, target: { pct: 10 } } });
  expect(host.textContent).toContain("fill at the next open");
  const target = [...host.querySelectorAll<HTMLElement>(".lab-order")].find((r) => r.textContent!.startsWith("Target"))!;
  await act(async () => [...target.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "Off")!.click());
  expect("target" in last().exit).toBe(false);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove Entry 1"]')!.click());
  expect(last().entry.all).toEqual([]);
  await unmount();
  const limited = await setup({ ...defaultSpec(), entry: { ...defaultSpec().entry, limit: { pct: 1 } } });
  expect(limited.host.textContent).toContain("fill at the limit price");
  expect(limited.host.textContent).not.toContain("Entry fill");
});

it("clearing the fees box keeps cost_bps and the same focused input", async () => {
  const onChange = vi.fn();
  const Host = () => {
    const [spec, setSpec] = useState<Spec>({ ...defaultSpec(), execution: { cost_bps: 10 } });
    return <RuleBuilder spec={spec} catalogue={cat} onChange={(s) => { onChange(s); setSpec(s); }}
      settings={settings} onSettings={() => {}} problems={[]} running={false} onRun={() => {}} />;
  };
  const host = await mount(<Host />);
  const fees = host.querySelector<HTMLInputElement>('input[aria-label="Fees (bps / side)"]')!;
  fees.focus();
  await typeIn(fees, "");
  expect(onChange.mock.calls.every(([s]) => (s as Spec).execution?.cost_bps !== undefined)).toBe(true);
  expect(host.querySelector('input[aria-label="Fees (bps / side)"]')).toBe(fees);
  expect(document.activeElement).toBe(fees);
  await typeIn(fees, "7");
  expect((onChange.mock.calls.at(-1)![0] as Spec).execution?.cost_bps).toBe(7);
});

it("each remove button and order input names its own row; repeated problems all render", async () => {
  const spec = addCond({ ...defaultSpec(), exit: { any: defaultSpec().exit.any, stop_loss: { pct: 5 }, target: { atr: 2, atr_length: 14 } } }, "entry", defaultSpec().entry.all[0]);
  const { host } = await setup(spec, { problems: ["same", "same"] });
  expect(host.querySelector('[aria-label="Remove Entry 2"]')).not.toBeNull();
  const names = [...host.querySelectorAll('button[aria-label^="Remove"]')].map((b) => b.getAttribute("aria-label"));
  expect(new Set(names).size).toBe(names.length);
  expect(host.querySelector('[role="group"][aria-label="Stop-loss"] input[aria-label="%"]')).not.toBeNull();
  expect(host.querySelector('[role="group"][aria-label="Target"] input[aria-label="ATR length"]')).not.toBeNull();
  expect(host.querySelectorAll(".rnote li")).toHaveLength(2);
  expect(host.textContent).toContain("Changing the entry fill also moves exits");
});

it("bars ago ignores fractions; a listed indicator with no computed ones keeps a disabled Indicator kind", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "or_range" } }] } } as Spec;
  const { host, onChange } = await setup(spec, { catalogue: { ...cat, indicators: cat.indicators.map((x) => ({ ...x, computed: false })) } });
  const rhsKind = host.querySelector<HTMLElement>('[aria-label="Entry 1 right"] select[aria-label="Operand type"]') as HTMLSelectElement;
  expect(rhsKind.value).toBe("indicator");
  expect(rhsKind.querySelector<HTMLOptionElement>('option[value="indicator"]')!.disabled).toBe(true);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Entry 1 left"] [data-lab-timing]')!.click());
  await typeIn(host.querySelector<HTMLInputElement>('[aria-label="Entry 1 left"] input[aria-label="bars ago"]')!, "1.5");
  expect(onChange).not.toHaveBeenCalled();
});

it("noncomparison nodes stay read-only and survive editing another rule", async () => {
  const node = { all: [{ lhs: { s: "rsi" }, op: "<", rhs: { value: 30 } }], future_tag: "keep" };
  const spec = { ...defaultSpec(), entry: { all: [node] }, future_rule: { enabled: true } } as Spec;
  const before = JSON.stringify(spec);
  const { host, last } = await setup(spec);
  expect(host.querySelector(".lab-cond .lab-kept")?.textContent).toContain(JSON.stringify(node));
  const exit = host.querySelectorAll<HTMLElement>(".lab-side")[1];
  await choose(exit.querySelector<HTMLSelectElement>('select[aria-label="Operator"]')!, "<=");
  expect(last().entry.all[0]).toEqual(node);
  expect(last().future_rule).toEqual({ enabled: true });
  expect(JSON.stringify(spec)).toBe(before);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove Entry 1"]')!.click());
  expect(last().entry.all).toEqual([]);
});


it("Phase7 Both exposes independent long and short rule lists and a compact caps line", async () => {
  const spec = { ...defaultSpec(), direction: "both", short_side: { entry: defaultSpec().entry, exit: defaultSpec().exit } } as Spec;
  const { host } = await setup(spec, { catalogue: catalogue7Fixture });
  expect([...host.querySelectorAll("[data-lab-side]")].map((e) => e.getAttribute("data-lab-side"))).toEqual(["long", "short"]);
  expect(host.querySelector("[data-lab-list='short_side.entry.all']")).not.toBeNull();
  expect(host.querySelector("[data-lab-caps]")!.textContent).toContain("comparisons 4/40");
  expect(host.querySelector("details[data-lab-risk]")).not.toBeNull();
});
it("Phase7 money uses declared costs instead of sending legacy fee fields", async () => {
  const spec = { ...defaultSpec(), execution: { capital: 100000 } } as Spec;
  const { host } = await setup(spec, { catalogue: catalogue7Fixture });
  expect(host.querySelector('input[aria-label="Fees (bps / side)"]')).toBeNull();
  expect(host.querySelector('input[aria-label="Slippage (bps / side)"]')).toBeNull();
});

// ---- AmiBroker Analysis settings tabs (General / Trades / Stops / Portfolio) and the Scenario presets
const tree = (spec: Spec, extra: Partial<Parameters<typeof RuleBuilder>[0]> = {}) => setup(spec, { catalogue: catalogue7Fixture, ...extra });
const tabs = (host: HTMLElement) => [...host.querySelectorAll<HTMLButtonElement>("[data-lab-settings-tabs] [role=tab]")];
const panel = (host: HTMLElement, label: string) => {
  const tab = tabs(host).find((t) => t.textContent === label)!;
  return [...host.querySelectorAll<HTMLElement>("[role=tabpanel]")].find((p) => p.id === tab.getAttribute("aria-controls"))!;
};
const scenario = (host: HTMLElement) => host.querySelector<HTMLSelectElement>('select[aria-label="Scenario"]')!;

it("settings tabs follow AmiBroker and the ARIA tab pattern; hidden panels stay mounted", async () => {
  const { host } = await tree(defaultSpec(), { portfolio: <p data-portfolio-slot>portfolio controls</p> });
  expect(tabs(host).map((t) => t.textContent)).toEqual(["General", "Trades", "Report", "Portfolio"]); // stops and targets sit with the sell rules
  expect(tabs(host).map((t) => t.tabIndex)).toEqual([0, -1, -1, -1]);
  expect(panel(host, "Report").querySelector("[data-lab-report-settings]")).not.toBeNull(); // risk-free rates on their own tab, as in AmiBroker
  for (const t of tabs(host)) expect(panel(host, t.textContent!).getAttribute("aria-labelledby")).toBe(t.id);
  // the basics come first, outside the tabs; every AmiBroker tab is folded under More settings, named when something is changed
  const basics = host.querySelector("[data-lab-basics]")!;
  expect([...basics.querySelectorAll(".lab-sgroup > h4")].map((h) => h.textContent)).toEqual(["Trade", "Test period & costs", "Starting money"]);
  expect(basics.querySelector('[data-lab-exec="positions"]')).not.toBeNull();
  expect(basics.querySelector("[data-lab-equity]")).not.toBeNull();
  expect(host.querySelector("[data-lab-more-settings] > summary")!.textContent).toBe("More settings (AmiBroker) · all at their defaults");
  expect(host.querySelector("[data-lab-more-settings] [role=tablist]")).not.toBeNull();
  expect([...panel(host, "General").querySelectorAll(".lab-sgroup > h4")].map((h) => h.textContent)).toEqual(["Position size & risk", "Trading options"]);
  expect(panel(host, "General").querySelector('[data-lab-exec="options"]')).not.toBeNull(); // same-bar options and instrument data
  expect(host.querySelector(".lab-settings-h")!.textContent).toContain("Backtest settings");
  expect(panel(host, "Trades").querySelector('[data-lab-trade-prices="preset"]')).not.toBeNull(); // grid always visible, greyed for presets
  expect(panel(host, "Portfolio").querySelector("[data-portfolio-slot]")).not.toBeNull();
  const list = host.querySelector<HTMLElement>("[role=tablist]")!;
  await act(async () => list.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(tabs(host)[1].getAttribute("aria-selected")).toBe("true");
  expect(document.activeElement).toBe(tabs(host)[1]);
  await act(async () => list.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })));
  expect(tabs(host)[3].tabIndex).toBe(0);
  await act(async () => list.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(tabs(host)[0].getAttribute("aria-selected")).toBe("true"); // wraps
  await act(async () => tabs(host)[2].click());
  expect(panel(host, "Trades").hidden).toBe(true);
  expect(panel(host, "Trades").querySelector('select[aria-label="Scenario"]')).not.toBeNull(); // hidden, not unmounted
});

it("Scenario presets rewrite the fills, keep Same-bar order, and show Custom for mixed settings", async () => {
  const { host, last } = await tree(defaultSpec());
  expect(scenario(host).value).toBe("next_open");
  await choose(scenario(host), "prices");
  expect(last().execution).toEqual({ prices: { buy: "open", sell: "open" }, delays: { buy: 1, sell: 1 } });
  const fromPrices = await tree(last());
  await choose(scenario(fromPrices.host), "close");
  expect(fromPrices.last().execution).toEqual({ fill: "close" });
  const ohlc = await tree({ ...defaultSpec(), execution: { intrabar: "ohlc_path" } });
  expect(scenario(ohlc.host).value).toBe("next_open"); // Same-bar order is not part of the scenario
  await choose(scenario(ohlc.host), "close");
  expect(ohlc.last().execution).toEqual({ fill: "close", intrabar: "ohlc_path" });
  const mixed = await tree({ ...defaultSpec(), execution: { exit_fill: "close" } });
  expect(scenario(mixed.host).value).toBe("custom");
  expect([...scenario(mixed.host).options].find((o) => o.value === "custom")!.textContent).toBe("Custom (see the fields below)");
  expect([...scenario(mixed.host).options].filter((o) => o.disabled)).toEqual([]);
  expect(scenario(mixed.host).textContent).not.toContain("not modelled");
});

it("stop scenarios set Exit at stop? of all four stops and are recognised back; mixed modes read Custom", async () => {
  const { host, last } = await tree(defaultSpec());
  await choose(scenario(host), "next_open_stops_open");
  expect(last().execution).toEqual({ exit_at_stop: { stop_loss: 2, target: 2, trailing: 2, bars: 2 } });
  expect(scenario((await tree(last())).host).value).toBe("next_open_stops_open");
  await choose(scenario(host), "close_stops_close");
  expect(last().execution).toEqual({ fill: "close", exit_at_stop: { stop_loss: 0, target: 0, trailing: 0, bars: 0 } });
  const back = await tree(last());
  expect(scenario(back.host).value).toBe("close_stops_close");
  await choose(scenario(back.host), "close"); // the intraday preset clears the modes
  expect(back.last().execution).toEqual({ fill: "close" });
  expect(scenario((await tree({ ...defaultSpec(), execution: { exit_at_stop: { target: 0 } } })).host).value).toBe("custom");
  expect(scenario((await tree({ ...defaultSpec(), execution: { exit_at_stop: { stop_loss: 0, target: 0, trailing: 0, bars: 0 } } })).host).value).toBe("custom"); // next open + stops on close
  const port = await tree({ ...defaultSpec(), execution: { capital: 1e6 }, portfolio: { universe: { basket: "nifty50" } } } as Spec);
  expect([...scenario(port.host).options].filter((o) => o.disabled).map((o) => o.value)).toEqual(["next_open_stops_open", "close_stops_close"]); // single-instrument modes
});

it("Activate stops immediately is a real toggle in the Trades tab whose default keeps today's behaviour, with a disclosure", async () => {
  const { host, last } = await tree(defaultSpec());
  const box = () => [...panel(host, "Trades").querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent === "Activate stops immediately")!.querySelector("input")!;
  expect([box().checked, box().disabled]).toEqual([true, false]);
  expect(panel(host, "Trades").textContent).toContain("AmiBroker's default is unchecked");
  await act(async () => box().click());
  expect(last().execution).toEqual({ activate_stops_immediately: false });
  const off = await tree(last());
  expect(panel(off.host, "Trades").textContent).toContain("Stops are first checked on the bar after the entry bar.");
});

it("Stops & targets under the sell rules have an N-bar stop; a portfolio hides per-side prices and re-entry and offers to remove kept ones", async () => {
  const { host, last } = await tree({ ...defaultSpec(), exit: { any: [] } });
  const stops = host.querySelector<HTMLElement>('[data-lab-list="exit.any"] [data-lab-orders-box="exits"]')!;
  expect(stops.querySelector("summary")!.textContent).toBe("Stops & targets · none (optional)");
  expect(host.querySelector('[data-lab-orders-box="entry"] summary')!.textContent).toContain("Order type"); // right under the buy rules
  const nbar = stops.querySelector<HTMLInputElement>('[data-lab-exit="n_bar"] input[aria-label="Bars held"]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(nbar, "4"); nbar.dispatchEvent(new Event("input", { bubbles: true })); });
  expect(last().exit.any).toEqual([{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 4 } }]);
  expect(host.querySelector('[data-lab-orders-box="exits"]')!.querySelectorAll('input[aria-label="Re-entry delay (bars)"]')).toHaveLength(4);
  const port = { ...defaultSpec(), execution: { capital: 1e6, reentry: { stop_loss: 2 }, prices: { buy: "close" as const }, delays: { buy: 0 } }, portfolio: { universe: { basket: "nifty50" } } } as Spec;
  const p = await tree(port);
  expect(p.host.querySelector('input[aria-label^="Re-entry delay"]')).toBeNull();
  expect(p.host.querySelector("[data-lab-trade-prices]")).toBeNull();
  expect([...scenario(p.host).options].find((o) => o.value === "prices")!.disabled).toBe(true);
  await act(async () => p.host.querySelector<HTMLButtonElement>("[data-lab-portfolio-stray] button")!.click());
  expect(p.last().execution).toEqual({ capital: 1e6 });
});

it("Filters tab: AFL-style filters are entry rules marked filter, each with its own sell-when-false box", async () => {
  const { CompactContext } = await import("../src/features/lab/simple");
  const onChange = vi.fn();
  const Host = () => {
    const [spec, setSpec] = useState<Spec>(defaultSpec());
    return <CompactContext.Provider value={true}><RuleBuilder spec={spec} catalogue={catalogue7Fixture} onChange={(s) => { onChange(s); setSpec(s); }}
      settings={settings} onSettings={() => {}} problems={[]} running={false} onRun={() => {}} part="rules" dates={<p data-dates-slot>dates</p>} /></CompactContext.Provider>;
  };
  const host = await mount(<Host />);
  const rules = () => [...host.querySelectorAll<HTMLElement>('[aria-label="Rules"] [role=tab]')];
  const pane = (label: string) => [...host.querySelectorAll<HTMLElement>("[role=tabpanel]")].find((p) => p.id === rules().find((t) => t.textContent === label)!.getAttribute("aria-controls"))!;
  const last = () => (onChange.mock.calls.at(-1)![0] as Spec).entry.all as Record<string, unknown>[];
  expect(rules().map((t) => t.textContent)).toEqual(["Entry · Buy", "Filters", "Exit · Sell"]);
  expect(pane("Filters").querySelector("[data-dates-slot]")).not.toBeNull(); // Base 1 / Base 2 windows with the filters
  await act(async () => pane("Filters").querySelector<HTMLButtonElement>("[data-lab-add-cond]")!.click());
  expect(last()).toHaveLength(2);
  expect(last()[1]).toMatchObject({ filter: true });
  await act(async () => pane("Filters").querySelector<HTMLInputElement>('input[aria-label="Filter 1: Also sell when it turns false"]')!.click());
  expect(last()[1]).toMatchObject({ filter: true, filter_exit: true });
  // the filter row's editor never sees the marks; editing it keeps them
  await choose(pane("Filters").querySelector<HTMLSelectElement>('select[aria-label="Operator"]')!, "<=");
  expect(last()[1]).toMatchObject({ op: "<=", filter: true, filter_exit: true });
  // a new entry row goes before the filters, so the signal rows keep their entry.all paths
  await act(async () => pane("Entry · Buy").querySelector<HTMLButtonElement>("[data-lab-add-cond]")!.click());
  expect(last().map((n) => n.filter === true)).toEqual([false, false, true]);
  expect(pane("Entry · Buy").querySelectorAll(".lab-cond")).toHaveLength(2);
  await act(async () => pane("Filters").querySelector<HTMLInputElement>('input[aria-label="Filter 1: Also sell when it turns false"]')!.click());
  expect(last()[2].filter_exit).toBeUndefined();
});
