import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ChartMatch } from "../src/features/lab/PlainRules";
import type { Catalogue, Spec } from "../src/features/lab/types";
import { catalogueFixture, mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const ind = (id: string, params: { key: string; def: number; choice?: boolean }[]) => ({ ...catalogueFixture.indicators[0], id, name: "Supertrend", computed: true,
  params: params.map((q) => ({ key: q.key, label: q.choice ? "Calculation (0 TradingView, 1 AmiBroker)" : q.key, default: q.def, min: 0, max: q.choice ? 1 : 500, step: 1, integer: true })) });
const cat = { ...catalogueFixture, indicators: [ind("supertrend", [{ key: "atr_length", def: 10 }, { key: "factor", def: 3 }, { key: "mode", def: 0, choice: true }])] } as unknown as Catalogue;
const st = (atr_length: number, factor: number, out = "direction") => ({ s: "supertrend", p: { atr_length, factor, mode: 1 }, out });
const spec = (entry: number[], exit: number[], dual?: number[]) => ({ v: 1, direction: "short",
  entry: { all: [{ lhs: st(entry[0], entry[1]), op: "crosses_above", rhs: { value: 0 } }] },
  exit: { any: [{ lhs: st(exit[0], exit[1]), op: "crosses_below", rhs: { value: 0 } }], ...(dual ? { dual_line: { initial: st(dual[0], dual[1], "supertrend"), favour: st(dual[0], dual[1] * 2, "supertrend") } } : {}) } }) as unknown as Spec;
const text = (h: HTMLElement) => h.querySelector("[data-lab-chart-match]")!.textContent!;

it("a changed Supertrend multiplier is named in full and can be set on the chart or added beside it", async () => {
  const onChartSet = vi.fn(), onChartAdd = vi.fn();
  const chips = [{ key: "c1", id: "supertrend", params: { atr_length: 5, factor: 1, mode: 1 } }];
  const host = await mount(<ChartMatch spec={spec([5, 1], [5, 2])} catalogue={cat} chips={chips} onSpec={vi.fn()} onChartSet={onChartSet} onChartAdd={onChartAdd} />);
  expect(text(host)).toContain("Supertrend 5, 1 ✓ same as the chart"); // the entry
  expect(text(host)).toContain("Chart Supertrend 5, 1, rule Supertrend 5, 2"); // the exit: the multiplier is visible
  const button = (label: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === label)!;
  await act(async () => button("Set the chart to Supertrend 5, 2").click());
  expect(onChartSet).toHaveBeenCalledWith("c1", { atr_length: 5, factor: 2, mode: 1 });
  await act(async () => button("Add Supertrend 5, 2 to the chart").click());
  expect(onChartAdd).toHaveBeenCalledWith(expect.objectContaining({ id: "supertrend" }), { atr_length: 5, factor: 2, mode: 1 });
});

it("the two-multiplier Supertrend exit lines are checked against the chart too", async () => {
  const host = await mount(<ChartMatch spec={spec([5, 1], [5, 1], [7, 1])} catalogue={cat} chips={[]} onSpec={vi.fn()} onChartSet={vi.fn()} onChartAdd={vi.fn()} />);
  expect(text(host)).toContain("Supertrend 7, 1 is not on the chart");
  expect(text(host)).toContain("Supertrend 7, 2 is not on the chart");
});
