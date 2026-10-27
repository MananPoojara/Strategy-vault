import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { NoTradesHelp } from "../src/features/lab/NoTrades";
import { DefaultExitCard, StrategyInWords } from "../src/features/lab/PlainRules";
import { RuleList } from "../src/features/lab/NodeEditor";
import { candleCountNode, starterSpec } from "../src/features/lab/spec-model";
import { DEFAULT_EXIT } from "../src/features/lab/defaults";
import type { ScanResult } from "../src/features/lab/client";
import type { RuleNode, Spec } from "../src/features/lab/types";
import { catalogue7Fixture as cat, flush, mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.unstubAllGlobals(); });
const rsi = { lhs: { s: "rsi", p: { length: 14 } }, op: "<", rhs: { value: 30 } } as RuleNode;
const ema = { lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } } as RuleNode;
const spec = (all: RuleNode[], patch: Partial<Spec> = {}): Spec => ({ ...starterSpec(), entry: { all }, ...patch });
/** Raw signal rows as /lab/scan answers them: one Buy row per signal day. */
const scanOf = (days: number): ScanResult => ({ kind: "lab_scan", label: "", apply: "current", from: null, to: null, notes: [], symbols: 1,
  rows: Array.from({ length: days }, (_, i) => ["NIFTY 50", `2020-01-${String(i + 1).padStart(2, "0")}`, "Buy", 1] as ScanResult["rows"][number]), refused: [] });

it("0 trades: rules that are each true but never on the same day are counted, and one click tests without one", async () => {
  vi.stubGlobal("fetch", () => Promise.reject(new Error("offline"))); // the value-range check is skipped offline
  const scan = vi.fn((s: Spec) => Promise.resolve(scanOf(s.entry.all.length > 1 ? 0 : JSON.stringify(s.entry.all[0]).includes("rsi") ? 249 : 4810)));
  const onFix = vi.fn();
  const host = await mount(<NoTradesHelp spec={spec([rsi, ema])} catalogue={cat} market="in" ticker="NIFTY 50" scan={scan} onFix={onFix} />);
  await flush(); await flush();
  const note = host.querySelector("[data-lab-no-trades]")!;
  expect(note.textContent).toContain("never true on the same day");
  expect(note.textContent).toContain("Rule 1 · RSI(14) is below 30: 249 days");
  expect(note.textContent).toContain("Rule 2 · the close is above the 50-day EMA: 4,810 days");
  expect(note.textContent).toContain("All of them together: 0 days");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-fix-off="1"]')!.click());
  expect(onFix.mock.calls[0][0](spec([rsi, ema])).entry.all[1]).toEqual({ ...ema, off: true });
});

it("0 trades with signal days: the settings that can stop a trade are listed with a one-click switch-off", async () => {
  vi.stubGlobal("fetch", () => Promise.reject(new Error("offline")));
  const onFix = vi.fn();
  const s = spec([rsi], { execution: { fill: "close", cooldown: { bars: 5 } } });
  const host = await mount(<NoTradesHelp spec={s} catalogue={cat} market="in" ticker="NIFTY 50" scan={() => Promise.resolve(scanOf(12))} onFix={onFix} />);
  await flush(); await flush();
  const note = host.querySelector("[data-lab-no-trades]")!;
  expect(note.textContent).toContain("true on 12 days, but no trade opened");
  expect(note.textContent).toContain("Cooldown: waits 5 bars after every trade");
  await act(async () => [...note.querySelectorAll("button")].find((b) => b.textContent === "Turn off and test again")!.click());
  expect(onFix.mock.calls[0][0](s).execution).toEqual({ fill: "close" });
});

it("plain words list switched-off rows and offer to turn off settings that block trades", async () => {
  const onChange = vi.fn();
  const s = spec([rsi, { ...ema, off: true } as RuleNode], { date_filter: { windows: [["2020-01-01", "2021-01-01"]] } });
  const host = await mount(<StrategyInWords spec={s} catalogue={cat} defaultExit={DEFAULT_EXIT} onChange={onChange} />);
  expect(host.querySelector("[data-lab-words-buy]")!.textContent).toBe("Buy when RSI(14) is below 30.");
  expect(host.querySelector("[data-lab-words-sell]")!.textContent).toContain("when the date window or a filter closes the trade");
  expect(host.querySelector("[data-lab-words-off]")!.textContent).toContain("the close is above the 50-day EMA");
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-words-blockers] button")!.click());
  expect(onChange.mock.calls[0][0].date_filter.off).toBe(true);
});

it("an Nth-candle exit reads as one sentence; its count and kind change in place", async () => {
  const onChange = vi.fn();
  const host = await mount(<RuleList list={[candleCountNode("up", 3)]} onChange={onChange} listPath="exit.any" title="Sell" tag="Exit" catalogue={cat}
    caps={{ leaves: false, nodes: false, top: false }} />);
  const row = host.querySelector('[data-lab-kind="candle"]')!;
  expect(row.textContent).toContain("Exit at the");
  expect(row.textContent).toContain("rd");
  await act(async () => {
    const kind = row.querySelector<HTMLSelectElement>('select[aria-label="Exit 1: candle"]')!;
    kind.value = "down"; kind.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(onChange.mock.calls.at(-1)![0]).toEqual([candleCountNode("down", 3)]);
  await act(async () => [...row.querySelectorAll("button")].find((b) => b.textContent === "Edit as condition")!.click());
  expect(host.querySelector('[data-lab-kind="candle"]')).toBeNull(); // the full condition editor
  expect(host.querySelector('[data-lab-operand-kind="since_entry"]')).not.toBeNull();
});

it("the default exit card switches parts on with a starting number, and keeps the last part on", async () => {
  const onChange = vi.fn();
  const host = await mount(<DefaultExitCard value={DEFAULT_EXIT} short={false} onChange={onChange} />);
  const box = (name: string) => host.querySelector<HTMLInputElement>(`input[aria-label="Default exit: use ${name}"]`)!;
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Default exit: days in the trade"]')!.disabled).toBe(true);
  await act(async () => box("days in the trade").click());
  expect(onChange.mock.calls.at(-1)![0]).toEqual({ up: 1, down: 4, days: 10 });
  await act(async () => box("a stop-loss").click());
  expect(onChange.mock.calls.at(-1)![0]).toEqual({ up: 1, down: 4, stopPct: 5 });
  await act(async () => box("the down close").click());
  expect(onChange.mock.calls.at(-1)![0]).toEqual({ up: 1, down: 4, downOff: true });
  await unmount();
  const one = await mount(<DefaultExitCard value={{ up: 1, down: 4, downOff: true }} short={false} onChange={onChange} />);
  expect(one.querySelector<HTMLInputElement>('input[aria-label="Default exit: use the up close"]')!.disabled).toBe(true);
});

it("a value read from another instrument names it in plain words", async () => {
  const foreign = { lhs: { s: "close", symbol: "NIFTYBANK" }, op: ">", rhs: { s: "ema", p: { length: 50 }, symbol: "NIFTYBANK" }, filter: true } as RuleNode;
  const host = await mount(<StrategyInWords spec={spec([rsi, foreign])} catalogue={cat} defaultExit={DEFAULT_EXIT} onChange={vi.fn()} />);
  expect(host.textContent).toContain("the close of NIFTYBANK is above the 50-day EMA of NIFTYBANK");
});
