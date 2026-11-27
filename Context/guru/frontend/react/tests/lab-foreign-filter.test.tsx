import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { ParamRule } from "../src/features/lab/ParamRule";
import { ForeignProvider } from "../src/features/lab/ForeignFields";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Catalogue, RuleNode, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const cat: Catalogue = { ...catalogue7Fixture, max_foreign: 4, execution: { ...catalogue7Fixture.execution!, entry_gates: ["dead_stock", "penny_price", "turnover_filter", "aux1", "aux2", "foreign_filter"] } };
const options = [{ ticker: "NIFTY 50", label: "NIFTY 50", disabled: false, hasVolume: false }, { ticker: "NIFTYBANK", label: "NIFTYBANK", disabled: false, hasVolume: false },
  { ticker: "INFY", label: "INFY", disabled: false, hasVolume: true, type: "individual_stock" }];
const settings = { fees: "", slippage: "", spread: "" } as never;

function Host({ start, onSpec }: { start: Spec; onSpec(s: Spec): void }) {
  const [spec, setSpec] = useState(start);
  return <ForeignProvider value={{ catalogue: cat, market: "in", ticker: "NIFTYBANK", options, spec }}>
    <RuleBuilder spec={spec} catalogue={cat} onChange={(s) => { onSpec(s); setSpec(s); }} settings={settings} onSettings={() => {}} problems={[]} running={false} onRun={() => {}} part="rules" />
  </ForeignProvider>;
}
const chip = (host: HTMLElement, t: string) => [...host.querySelectorAll<HTMLButtonElement>(".lab-filter-chip")].find((b) => b.textContent === t)!;
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });

it("Filters › Foreign filter: the AFL's ST For Foreign Filter panel; defaults stay out of the spec; No filter clears it", async () => {
  const onSpec = vi.fn();
  const host = await mount(<Host start={defaultSpec()} onSpec={onSpec} />);
  expect(chip(host, "Foreign filter").getAttribute("aria-pressed")).toBe("false");
  await act(async () => chip(host, "Foreign filter").click());
  const last = () => onSpec.mock.lastCall![0] as Spec;
  expect(last().execution?.foreign_filter).toEqual({ symbol: "NIFTY 50" }); // the AFL's defaults are implied
  const panel = host.querySelector("[data-lab-foreign-filter]")!;
  expect([...panel.querySelectorAll(".lab-param-k")].map((k) => k.textContent)).toEqual(["Ticker", "Timeframe", "ST period", "ST factor 1", "ST factor 2", "Average value", "Type", "Exit"]);
  const sel = (label: string) => panel.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`)!;
  expect([...sel("Foreign filter ticker").options].map((o) => o.value).slice(0, 2)).toEqual(["NIFTY 50", "NIFTYBANK"]); // indices first
  expect(sel("Foreign filter timeframe").value).toBe("M");
  expect(sel("Foreign filter average value").selectedOptions[0].text).toBe("(High+Low)/2");
  await choose(sel("Foreign filter timeframe"), "W");
  await choose(sel("Foreign filter exit"), "yes");
  expect(last().execution?.foreign_filter).toEqual({ symbol: "NIFTY 50", tf: "W", exit: true });
  await choose(sel("Foreign filter timeframe"), "M"); // back to the default: left out again
  expect(last().execution?.foreign_filter).toEqual({ symbol: "NIFTY 50", exit: true });
  await act(async () => chip(host, "No filter").click());
  expect(last().execution?.foreign_filter).toBeUndefined();
  expect(host.querySelector("[data-lab-foreign-filter]")).toBeNull();
});

it("an engine without the foreign filter shows no Foreign filter choice", async () => {
  const plain = { ...cat, execution: { ...cat.execution!, entry_gates: ["dead_stock"] } };
  const host = await mount(<ForeignProvider value={{ catalogue: plain, market: "in", ticker: "NIFTYBANK", options }}>
    <RuleBuilder spec={defaultSpec()} catalogue={plain} onChange={vi.fn()} settings={settings} onSettings={() => {}} problems={[]} running={false} onRun={() => {}} part="rules" />
  </ForeignProvider>);
  expect(chip(host, "Foreign filter")).toBeUndefined();
});

it("a portfolio's rules offer Foreign ticker, read on each member (Current member)", async () => {
  const host = await mount(<ForeignProvider value={{ catalogue: cat, market: "in", ticker: "", options, portfolio: true }}>
    <ParamRule node={{ lhs: { s: "close" }, op: ">", rhs: { value: 1 } } as RuleNode} catalogue={cat} label="Entry 1" onChange={vi.fn()} onLhs={vi.fn()} />
  </ForeignProvider>);
  const pick = host.querySelector<HTMLSelectElement>('select[aria-label="Entry 1 check symbol"]')!;
  expect(pick.options[0].text).toBe("Current member");
  expect([...pick.options].map((o) => o.value)).toContain("NIFTY 50");
});
