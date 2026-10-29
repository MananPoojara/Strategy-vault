import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { defaultSpec, operandLabel } from "../src/features/lab/spec-model";
import type { Catalogue, RuleNode, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const base = catalogue7Fixture.indicators[0];
const st = { ...base, id: "supertrend", name: "Supertrend", category: "Trend", outputs: ["supertrend", "direction"], params: [
  { key: "atr_length", label: "ATR length", default: 10, min: 1, max: 500, step: 1, integer: true },
  { key: "factor", label: "Factor", default: 3, min: 0.5, max: 20, step: 0.5, integer: false },
  { key: "mode", label: "Mode (0 TradingView, 1 AmiBroker)", default: 0, min: 0, max: 1, step: 1, integer: true },
  { key: "price", label: "Price (0 (H+L)/2, 1 Close, 2 Open, 3 High, 4 Low, 5 (O+C)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", default: 0, min: 0, max: 7, step: 1, integer: true }] };
const cat: Catalogue = { ...catalogue7Fixture, indicators: [...catalogue7Fixture.indicators, st] };

it("a Supertrend picked in a rule starts at the AFL's 5 × 1 AmiBroker (H+L)/2, labelled as in the Exit card; plain words show only the numbers", async () => {
  expect(operandLabel({ s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 } }, cat)).toBe("Supertrend(5, 1)");
  const { ruleWords } = await import("../src/features/lab/spec-model");
  expect(ruleWords({ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 }, offset: 1 } } as unknown as RuleNode, cat)!.text)
    .toBe("the close crosses above Supertrend(5, 1) 1 day ago");
  const onChange = vi.fn();
  const start: Spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 20 } } } as unknown as RuleNode] } };
  function Host() {
    const [spec, setSpec] = useState(start);
    return <RuleBuilder spec={spec} catalogue={cat} onChange={(s) => { onChange(s); setSpec(s); }} settings={{ from: "", to: "", fees: "", slippage: "" }}
      onSettings={() => {}} problems={[]} running={false} onRun={() => {}} />;
  }
  const host = await mount(<Host />), last = () => onChange.mock.calls.at(-1)![0] as Spec;
  const card = () => host.querySelector<HTMLElement>("[data-lab-card=entry]")!;
  const indRow = [...card().querySelectorAll(".lab-param-row")].find((r) => r.querySelector(".lab-param-k")!.textContent === "Indicator")!;
  await act(async () => indRow.querySelector("button")!.click());
  await act(async () => { card().querySelector('li[data-id="supertrend"]')!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })); });
  expect((last().entry.all[0] as { rhs: { p: unknown } }).rhs.p).toEqual({ atr_length: 5, factor: 1, mode: 1, price: 0 });
  const labels = [...card().querySelectorAll(".lab-param-k")].map((k) => k.textContent);
  expect(labels).toEqual(expect.arrayContaining(["Period", "Multiplier", "Calculation", "Avg Val"]));
  expect(labels).not.toContain("ATR length");
});
