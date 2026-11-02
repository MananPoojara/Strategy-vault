import { afterEach, expect, it } from "vitest";
import { act } from "react";
import { WalkForwardTab } from "../src/features/lab/settings/WalkForwardTab";
import { MonteCarloTab } from "../src/features/lab/settings/MonteCarloTab";
import { MonteCarloPanel } from "../src/features/lab/MonteCarlo";
import { RUN_PREFS_DEFAULTS, resetRunPrefs, useRunPrefs } from "../src/features/lab/settings/run-prefs";
import { defaultSpec } from "../src/features/lab/spec-model";
import { CASH_ONLY } from "../src/features/lab/settings/common";
import type { Trade } from "../src/features/lab/types";
import { mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); act(resetRunPrefs); });
const set = async (el: Element, value: string) => act(async () => {
  const proto = el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
});
const q = <T extends Element = HTMLInputElement>(host: Element, label: string) => host.querySelector<T & Element>(`[aria-label="${label}"]`)!;
const days = Array.from({ length: 365 * 7 }, (_, i) => new Date(Date.UTC(2015, 0, 1) + i * 86400e3).toISOString().slice(0, 10));
let seen = RUN_PREFS_DEFAULTS;
const Spy = () => { seen = useRunPrefs(); return null; };

it("walk-forward tab: the sheet's defaults, unsupported modes disabled with the reason, and the window list from the bars", async () => {
  const host = await mount(<><WalkForwardTab spec={defaultSpec()} dates={days} /><Spy /></>);
  expect([q(host, "In-sample Length").value, q(host, "Out-of-sample Length").value, q(host, "In-sample Step").value]).toEqual(["4", "1", "1"]);
  expect(q<HTMLSelectElement>(host, "Walk-forward target").value).toBe("mar");
  // only easy mode (EOD) exists in the lab, so the unsupported modes are left out
  for (const m of ["Easy mode (intraday)", "Advanced mode"]) expect(host.querySelector(`[aria-label="${m}"]`)).toBeNull();
  expect(q(host, "In-sample Start").value).toBe("01-01-2015");
  expect(q(host, "Out-of-sample Start").disabled).toBe(true);
  expect([...host.querySelectorAll("[data-lab-wf-preview] tbody tr")].map((r) => r.children[0].textContent).slice(0, 4)).toEqual(["IS", "OOS", "IS", "OOS"]);
  await set(q(host, "In-sample Length"), "2");
  expect(seen.wf.is).toBe(2);
  // a step shorter than the out-of-sample window stays local: the store keeps the last runnable options
  await set(q(host, "Out-of-sample Length"), "3");
  expect(host.querySelector("[data-lab-wf-bad]")).not.toBeNull();
  expect(seen.wf.oos).toBe(1);
});

it("walk-forward tab explains the dates before a backtest", async () => {
  const host = await mount(<WalkForwardTab spec={defaultSpec()} />);
  expect(q(host, "In-sample Start").value).toBe("");
  expect(host.textContent).toContain("Run a backtest first to preview the windows.");
});

const t = (ret: number) => ({ ret, open: false, bars: 3, dir: 1 }) as Trade;
it("monte carlo tab: the sheet's defaults, cash-only sizes disabled in % returns, and the panel follows it", async () => {
  const host = await mount(<><MonteCarloTab spec={defaultSpec()} /><MonteCarloPanel trades={[t(0.1), t(-0.05), t(0.02)]} from="2020-01-01" to="2022-01-01" /></>);
  const tab = host.querySelector("[data-lab-mc-tab]")!, panel = host.querySelector("[data-lab-monte-carlo]")!;
  expect([q(tab, "Enable Monte Carlo simulation").checked, q(tab, "Number of runs").value, q(tab, "Simulate using trade list").checked, q(tab, "Don't change").checked]).toEqual([true, "1000", true, true]);
  expect([q(tab, "Enable MC Equity curves (Min/Max/Avg)").checked, q(tab, "Show Absolute values in linear scale").checked, q(tab, "Use negative numbers for Drawdown (reverse Drawdown CDF)").checked]).toEqual([true, true, true]);
  for (const l of ["Simulate using portfolio equity changes", "Straw broom chart plots", "Use logarithmic scale for $ Drawdown"]) expect(q(tab, l).disabled).toBe(true);
  expect(q(tab, "Fixed size").disabled).toBe(true);
  expect(q(tab, "Fixed size").closest(".lab-row")!.textContent).toContain(CASH_ONLY);
  await set(q(tab, "Number of runs"), "200");
  expect(q(panel, "Runs").value).toBe("200");
  await act(async () => q(tab, "Percent of Equity").click());
  expect(q<HTMLSelectElement>(panel, "Sizing").value).toBe("fraction");
  // positive drawdowns: the worst moves to the 99% row
  await act(async () => q(tab, "Use negative numbers for Drawdown (reverse Drawdown CDF)").click());
  await act(async () => q(tab, "Enable MC Equity curves (Min/Max/Avg)").click());
  const run = [...panel.querySelectorAll("button")].find((b) => b.textContent === "Run Monte Carlo")!;
  await act(async () => run.click());
  const dd = [...panel.querySelectorAll("[data-lab-mc-table] tbody tr")].map((r) => parseFloat(r.children[4].textContent!));
  expect(dd.every((d) => d >= 0)).toBe(true);
  expect(dd[0]).toBeLessThanOrEqual(dd[dd.length - 1]);
  expect(panel.querySelector("[data-lab-mc-chart=envelope]")).toBeNull();
  await act(async () => q(tab, "Enable Monte Carlo simulation").click());
  expect(run.disabled).toBe(true);
  expect(panel.querySelector("[data-lab-mc-note]")!.textContent).toContain("Monte Carlo is off");
});
