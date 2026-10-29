import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { bandWords, buildBand, defaultBand, parseBand } from "../src/features/lab/BandRule";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Catalogue, RuleNode, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const base = catalogue7Fixture.indicators[0];
const cat: Catalogue = { ...catalogue7Fixture, timeframes: ["D", "W", "M"], indicators: [...catalogue7Fixture.indicators,
  { ...base, id: "bb", name: "Bollinger Bands", category: "Volatility", outputs: ["basis", "upper", "lower"],
    params: [{ key: "length", label: "Length", default: 20, min: 1, max: 500, step: 1, integer: true }, { key: "mult", label: "Multiplier", default: 2, min: 0.1, max: 10, step: 0.1, integer: false }] },
  { ...base, id: "roc", name: "Rate of Change", category: "Momentum", placement: "pane", outputs: ["roc"], params: [{ key: "length", label: "Length", default: 9, min: 1, max: 500, step: 1, integer: true }] }] };
const bb = { s: "bb", p: { length: 5, mult: 1 }, tf: "M" };

it("the default entry is the AFL's: close below the lower band, then a green candle back above the previous bar's lower band, under the upper band, at month end", () => {
  const n = defaultBand({ ...bb, out: "lower" }, false);
  const M = { tf: "M" }, line = (out: string, offset?: number) => ({ ...bb, out, ...(offset ? { offset } : {}) });
  expect(n).toEqual({
    setup: { lhs: { s: "close", ...M }, op: "<", rhs: line("lower") },
    trigger: { all: [{ lhs: { s: "close", ...M }, op: ">", rhs: { s: "open", ...M } }, { lhs: { s: "close", ...M }, op: ">", rhs: line("lower", 1) }, { lhs: { s: "close", ...M }, op: "<", rhs: line("upper") }] },
    cancel: { lhs: { s: "close", ...M }, op: ">", rhs: line("upper") }, period_end: "M",
  });
  const b = parseBand(n, cat)!;
  expect(b).toMatchObject({ from: "lower", dir: "below", candle: "green", prev: true, inside: true, cancel: "upper", periodEnd: true, extra: [] });
  // the exit mirrors it from the upper band; any choice round-trips, a hand-written setup does not parse
  const exit = defaultBand({ ...bb, tf: "D", out: "upper" }, true);
  expect(parseBand(exit, cat)).toMatchObject({ from: "upper", dir: "above", candle: "red", cancel: "lower", periodEnd: false });
  for (const patch of [{ from: "basis" }, { candle: "any" as const }, { prev: false }, { inside: false }, { cancel: "none" as const }, { extra: [{ lhs: { s: "roc", p: { length: 1 } }, op: "<=", rhs: { value: -2 } } as RuleNode] }])
    expect(parseBand(buildBand({ ...b, ...patch }), cat)).toEqual({ ...b, ...patch });
  expect(parseBand({ setup: { lhs: { s: "close" }, op: "<", rhs: { s: "ema" } }, trigger: { lhs: { s: "close" }, op: ">", rhs: { s: "open" } } } as unknown as RuleNode, cat)).toBeUndefined();
  expect(bandWords(n, cat)).toBe("after a close below the monthly Bollinger Bands (5, 1) lower band, a green candle closes back above it (previous bar's value), still below the upper band; a close above the upper band cancels it (at month end)");
});

it("a rule with a band offers Band setup; it edits as one list and Simple compare turns it back into a crossing", async () => {
  const onChange = vi.fn();
  const start: Spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: "crosses_below", rhs: { ...bb, tf: undefined, out: "lower" } } as unknown as RuleNode] } };
  function Host() {
    const [spec, setSpec] = useState(start);
    return <RuleBuilder spec={spec} catalogue={cat} onChange={(s) => { onChange(s); setSpec(s); }} settings={{ from: "", to: "", fees: "", slippage: "" }}
      onSettings={() => {}} problems={[]} running={false} onRun={() => {}} />;
  }
  const host = await mount(<Host />), last = () => onChange.mock.calls.at(-1)![0] as Spec;
  const button = (text: string) => [...host.querySelectorAll<HTMLButtonElement>("[data-lab-card=entry] button")].find((b) => b.textContent === text)!;
  await act(async () => button("Band setup").click());
  expect(Object.keys(last().entry.all[0])).toEqual(["setup", "trigger", "cancel"]); // daily: no period_end
  expect(host.querySelector("[data-lab-card=entry] [data-lab-band]")).not.toBeNull();
  const candle = host.querySelector<HTMLSelectElement>('select[aria-label="Entry 1 candle"]')!;
  await act(async () => { candle.value = "any"; candle.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(parseBand(last().entry.all[0], cat)).toMatchObject({ candle: "any", from: "lower" });
  await act(async () => (host.querySelector("[data-lab-band-extra]") as HTMLButtonElement).click());
  expect(parseBand(last().entry.all[0], cat)!.extra).toHaveLength(1);
  await act(async () => (host.querySelector("[data-lab-band-simple]") as HTMLButtonElement).click());
  expect(last().entry.all[0]).toEqual({ lhs: { s: "close" }, op: "crosses_below", rhs: { s: "bb", p: { length: 5, mult: 1 }, out: "lower" } });
});

it("Bollinger %B opens the same setup: %B below its level, then back above the Bollinger line (the AFL's %B < 0)", () => {
  const withPct: Catalogue = { ...cat, indicators: [...cat.indicators, { ...base, id: "bbpct", name: "Bollinger %B", category: "Volatility", placement: "pane", outputs: ["pct_b"], params: cat.indicators.find((i) => i.id === "bb")!.params }] };
  const n = defaultBand({ s: "bbpct", p: { length: 5, mult: 1 }, tf: "M" }, false) as unknown as Record<string, Record<string, unknown>>;
  expect(n.setup).toEqual({ lhs: { s: "bbpct", p: { length: 5, mult: 1 }, tf: "M" }, op: "<", rhs: { value: 0 } });
  expect((n.trigger.all as unknown[])[1]).toEqual({ lhs: { s: "close", tf: "M" }, op: ">", rhs: { s: "bb", p: { length: 5, mult: 1 }, tf: "M", out: "lower", offset: 1 } });
  const b = parseBand(n as unknown as RuleNode, withPct)!;
  expect(b).toMatchObject({ pct: 0, from: "lower", ind: { s: "bb", p: { length: 5, mult: 1 }, tf: "M" } });
  expect(parseBand(buildBand({ ...b, pct: -0.2, from: "basis" }), withPct)).toMatchObject({ pct: -0.2, from: "basis" });
  expect(bandWords(n as unknown as RuleNode, withPct)).toMatch(/^after monthly %B \(5, 1\) below 0, a green candle closes back above the monthly Bollinger Bands \(5, 1\) lower band/);
});
