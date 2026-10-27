import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import type { SeriesResponse } from "../src/features/lab/types";

// Ulcer-like pane series 0.5…20 (typical 4.2); the price close and its averages ≈ 20,000
const ranges: Record<string, number[]> = { obv: [0.5, 4.2, 4.2, 20], close: [19000, 20000, 21000], ema: [18000, 19500, 21000] };
vi.mock("../src/features/lab/client", () => ({
  series: vi.fn(async (_m: string, q: { id: string }) => ({ outputs: ["v"], dates: [], series: { v: ranges[q.id] ?? [] } }) as unknown as SeriesResponse),
}));
const { RuleList } = await import("../src/features/lab/NodeEditor");
const { ForeignProvider } = await import("../src/features/lab/ForeignFields");
const { canMeet, neverMeets } = await import("../src/features/lab/scale-check");
const { catalogue7Fixture, mount, unmount, pickIndicator } = await import("./fixtures/lab");
import type { Cond, RuleNode, Spec } from "../src/features/lab/types";

afterEach(unmount);
const MA: Cond = { lhs: { s: "ema", p: { length: 9 } }, op: "crosses_above", rhs: { s: "ema", p: { length: 50 } } };
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const setup = async (scoped: boolean) => {
  const onChange = vi.fn();
  // the page re-renders with each change, as here
  function Rules() {
    const [list, setList] = useState<RuleNode[]>([MA]);
    return <RuleList list={list} onChange={(l) => { onChange(l); setList(l); }} listPath="entry.all" title="Entry" tag="Entry" catalogue={catalogue7Fixture} caps={{ leaves: false, nodes: false, top: false }} />;
  }
  const host = await mount(scoped ? <ForeignProvider value={{ catalogue: { tickers: [] } as never, ticker: "NIFTY 50", market: "in", options: [] }}><Rules /></ForeignProvider> : <Rules />);
  return { host, calls: () => onChange.mock.calls.map(([l]) => (l as RuleNode[])[0] as Cond) };
};

it("each indicator picked on the left gets the right side it is read against", async () => {
  const pick = async (id: string) => {
    await unmount();
    const { host, calls } = await setup(false);
    await pickIndicator(host.querySelector('[aria-label="Entry 1 left"]')!, id);
    return calls().at(-1)!;
  };
  expect(await pick("rsi")).toMatchObject({ lhs: { s: "rsi" }, rhs: { value: 70 } }); // its usual level
  expect(await pick("macd")).toMatchObject({ lhs: { s: "macd" }, rhs: { s: "macd", out: "signal" } }); // its own signal line
  const { host, calls } = await setup(false);
  const left = () => host.querySelector('[aria-label="Entry 1 left"]')!;
  await pickIndicator(left(), "rsi");
  await pickIndicator(left(), "ema");
  expect(calls().at(-1)).toMatchObject({ lhs: { s: "ema" }, rhs: { s: "close" } }); // a moving average against the price
});

it("without a usual level the right side becomes the indicator's typical value on this instrument", async () => {
  const { host, calls } = await setup(true);
  await pickIndicator(host.querySelector('[aria-label="Entry 1 left"]')!, "obv");
  await act(async () => { await new Promise((r) => setTimeout(r, 0)); });
  expect(calls().map((c) => c.rhs)).toEqual([{ value: 0 }, { value: 4.2 }]);
});

it("a second indicator on the right stays the user's choice; a new length keeps the right side", async () => {
  const { host, calls } = await setup(false);
  await choose(host.querySelector('[aria-label="Entry 1 right"] select')!, "indicator");
  const rhs = calls().at(-1)!.rhs;
  expect(rhs).toMatchObject({ s: expect.any(String) });
  await act(async () => {
    const len = host.querySelector<HTMLInputElement>('[aria-label="Entry 1 left"] input[type="number"]')!;
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(len, "21");
    len.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(calls().at(-1)).toMatchObject({ lhs: { s: "ema" }, rhs });
});

it("a 0-trade run names the buy rule whose sides never meet", async () => {
  expect(canMeet("crosses_above", { min: 0.5, max: 20, typical: 4 }, { min: 18000, max: 21000, typical: 19500 })).toBe(false);
  expect(canMeet("crosses_above", { min: 0.5, max: 20, typical: 4 }, { min: 5, max: 5, typical: 5 })).toBe(true);
  expect(canMeet(">", { min: 0.5, max: 20, typical: 4 }, { min: 0, max: 0, typical: 0 })).toBe(true);
  const spec: Spec = { v: 1, entry: { all: [{ lhs: { s: "obv" }, op: "crosses_above", rhs: { s: "ema", p: { length: 50 } } }, { lhs: { s: "obv" }, op: ">", rhs: { value: 25 } },
    { lhs: { s: "obv" }, op: ">", rhs: { value: 2 } }] }, exit: { any: [] } } as Spec;
  const notes = await neverMeets(spec, catalogue7Fixture, "in", "NIFTY 50");
  expect(notes).toHaveLength(2);
  expect(notes[0]).toMatch(/^Buy rule 1 is never true: .*0\.5 to 20 on NIFTY 50.*18,000 to 21,000.*compare it with a number, e\.g\. 4\.2\.$/);
  expect(notes[1]).toMatch(/^Buy rule 2 is never true: .*never reaches 25\. To fix it, use a number between 0\.5 and 20/);
});
