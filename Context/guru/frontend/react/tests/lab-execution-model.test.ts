import { expect, it } from "vitest";
import * as x from "../src/features/lab/execution-model";
import { COPY, FUTURES_NOTE, MODEL_NOTE, MONEY_BLOCKED } from "../src/features/lab/labels";
import { defaultSpec, setAt, setExecution } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogue7Fixture as unitCat } from "./fixtures/lab";

const cat = { ...unitCat, execution: { ...unitCat.execution!, sizing: ["pct_equity", "risk_pct", "units", "value"] } };

const idx: x.InstrumentContext = { kind: "index", market: "in" };
const inStock: x.InstrumentContext = { kind: "individual_stock", market: "in" };
const usEtf: x.InstrumentContext = { kind: "etf", market: "us" };

it("direction: both adds a short side and keeps it across a round trip; long removes short_side and opposite", () => {
  const s = setExecution(defaultSpec(), { opposite: "exit" });
  const both = x.withDirection(s, "both");
  expect(both.direction).toBe("both");
  expect(both.short_side).toEqual(x.DEFAULT_SHORT_SIDE);
  const edited = setAt(both, "short_side.entry.all[0].rhs.p.length", 20);
  const long = x.withDirection(edited, "long");
  expect(long).not.toHaveProperty("short_side");
  expect(long).not.toHaveProperty("direction");
  expect(long.execution).toBeUndefined();
  expect(x.withDirection(long, "both", edited.short_side).short_side?.entry.all[0]).toMatchObject({ rhs: { p: { length: 20 } } });
  expect(x.withDirection(defaultSpec(), "short").direction).toBe("short");
  expect(x.hasShorts(both) && x.hasShorts(x.withDirection(s, "short")) && !x.hasShorts(s)).toBe(true);
});

it("entry orders: types, default levels, fill moved to exit_fill, per side", () => {
  const s = setExecution(defaultSpec(), { fill: "close" });
  const stop = x.withEntryOrder(s, "", "stop");
  expect(stop.entry.stop).toEqual({ pct: 1 });
  expect(stop.execution).toEqual({ exit_fill: "close" });
  expect(x.entryOrderType(stop.entry)).toBe("stop");
  const sl = x.withEntryOrder(stop, "", "stop_limit");
  expect([sl.entry.limit, sl.entry.stop, x.entryOrderType(sl.entry)]).toEqual([{ pct: 1 }, { pct: 1 }, "stop_limit"]);
  const market = x.withEntryOrder(setAt(sl, "entry.valid_bars", 3), "", "market");
  expect(market.entry).not.toHaveProperty("limit");
  expect(market.entry).not.toHaveProperty("valid_bars");
  const both = x.withDirection(defaultSpec(), "both");
  expect(x.withEntryOrder(both, "short_side.", "limit").short_side?.entry.limit).toEqual({ pct: 1 });
  expect(x.defaultLevel("entry.stop", "at", false)).toEqual({ at: { highest: { s: "high" }, bars: 5 } });
  expect(x.defaultLevel("entry.stop", "at", true)).toEqual({ at: { lowest: { s: "low" }, bars: 5 } });
  expect(x.defaultLevel("entry.limit", "at", false)).toEqual({ at: { lowest: { s: "low" }, bars: 5 } });
  expect(x.defaultLevel("exit.stop_loss", "atr", false)).toEqual({ atr: 2, atr_length: 14 });
  expect(x.defaultLevel("exit.target", "points", false)).toEqual({ points: 10 });
});

it("levels: trailing keeps its activation; breakeven after removal drops breakeven; take-profits list", () => {
  let s = x.withLevel(defaultSpec(), "", "exit.trailing", { pct: 5 });
  s = x.withLevel(s, "", "exit.trailing.activate", { pct: 2 });
  s = x.withLevel(s, "", "exit.trailing", { atr: 3, atr_length: 10 });
  expect(s.exit.trailing).toEqual({ atr: 3, atr_length: 10, activate: { pct: 2 } });
  expect(x.levelAt(s, "", "exit.trailing")).toEqual({ atr: 3, atr_length: 10 });
  expect(x.withLevel(s, "", "exit.trailing.activate", undefined).exit.trailing).toEqual({ atr: 3, atr_length: 10 });
  s = x.withLevel(s, "", "exit.breakeven.after", { pct: 2 });
  s = x.withLevel(s, "", "exit.breakeven.plus", { pct: 0.1 });
  expect(s.exit.breakeven).toEqual({ after: { pct: 2 }, plus: { pct: 0.1 } });
  expect(x.withLevel(s, "", "exit.breakeven.after", undefined).exit).not.toHaveProperty("breakeven");
  s = x.withTakeProfits(s, "", [{ pct: 3, size: 0.5 }]);
  expect(x.takeProfits(s, "")).toEqual([{ pct: 3, size: 0.5 }]);
  expect(x.withTakeProfits(s, "", []).exit).not.toHaveProperty("take_profits");
});

it("money mode: capital default, removes cost_bps; unit mode drops money-only sizing and costs", () => {
  const s = setExecution(defaultSpec(), { cost_bps: 10, sizing: { units: 5 } });
  const money = x.withMode(s, "money");
  expect(money.execution).toEqual({ capital: 1_000_000, sizing: { units: 5 } });
  expect(x.modeOf(money)).toBe("money");
  const unit = x.withMode(setExecution(money, { costs: { slippage: { bps: 5 } } }), "unit");
  expect(unit.execution).toBeUndefined();
  expect(x.withMode(setExecution(money, { sizing: { risk_pct: 1 } }), "unit").execution).toEqual({ sizing: { risk_pct: 1 } });
  expect(x.sizingKind(defaultSpec())).toBe("full");
  expect(x.withSizingKind(defaultSpec(), "risk_pct").execution).toEqual({ sizing: { risk_pct: 1 } });
  expect(x.withSizingKind(x.withSizingKind(defaultSpec(), "pct_equity"), "full").execution).toBeUndefined();
});

it("instrument context, money gating and pre-run notes mirror model.rs", () => {
  expect(x.instrumentContext("in", undefined)).toEqual({ kind: "index", market: "in" });
  expect(x.instrumentContext("us", { ticker: "SPY", label: "SPY", disabled: false, type: "etf", hasVolume: true })).toEqual({ kind: "etf", market: "us" });
  expect(x.instrumentContext("gl", { ticker: "IG:DAX", label: "DAX", disabled: false, type: "cfd", hasVolume: false })).toEqual({ kind: "cfd", market: "gl" });
  const short = x.withDirection(defaultSpec(), "short");
  expect(x.moneyBlocked(idx, defaultSpec())).toBe(MONEY_BLOCKED.index);
  expect(x.moneyBlocked({ kind: "crypto", market: "gl" }, defaultSpec())).toBe(MONEY_BLOCKED.crypto);
  expect(x.moneyBlocked({ kind: "cfd", market: "gl" }, defaultSpec())).toBe(MONEY_BLOCKED.cfd);
  expect(x.moneyBlocked(inStock, short)).toBe(MONEY_BLOCKED.indiaShort);
  expect(x.moneyBlocked(inStock, defaultSpec())).toBeNull();
  expect(x.moneyBlocked(usEtf, short)).toBeNull();
  expect(x.instrumentNotes(idx, short)).toEqual([MODEL_NOTE.index]);
  expect(x.instrumentNotes(inStock, short)).toEqual([MODEL_NOTE.indiaShort]);
  expect(x.instrumentNotes(usEtf, short)).toEqual([MODEL_NOTE.usShortUnit]);
  expect(x.instrumentNotes(usEtf, x.withMode(short, "money"))).toEqual([]);
  expect(x.instrumentNotes({ kind: "crypto", market: "gl" }, short)).toEqual([MODEL_NOTE.crypto, MODEL_NOTE.cryptoShort]);
  expect(x.instrumentNotes(usEtf, defaultSpec())).toEqual([]);
});

it("execution problems: distances, take-profits, breakeven, orders, scale-in, sizing, money, re-entry, futures", () => {
  const p = (s: Spec, ctx = usEtf) => x.executionProblems(s, cat, ctx);
  const s = defaultSpec();
  expect(p(s)).toEqual([]);
  expect(p(x.withLevel(s, "", "exit.stop_loss", { pct: 0 }))).toContain("Stop-loss: % must be above 0 and at most 99.");
  expect(p(x.withLevel(s, "", "entry.limit", { pct: 0 }))).toEqual([]);
  expect(p(x.withLevel(s, "", "exit.target", { atr: 51 }))).toContain("Target: ATR multiplier must be above 0 and at most 50.");
  expect(p(x.withLevel(s, "", "exit.target", { atr: 2, atr_length: 600 }))).toContain("Target: ATR length must be a whole number from 1 to 500.");
  expect(p(setAt(s, "exit.stop_loss", { ticks: 20 }))).toContain("Stop-loss: ticks need a futures contract, and futures contract data is not admitted.");
  expect(p(setAt(s, "exit.target", { at: { s: "high" } }))).toContain("Target: an At level works only for entry orders.");
  const both = x.withDirection(s, "both");
  expect(p(x.withLevel(both, "short_side.", "exit.stop_loss", { pct: 120 }))).toContain("Short Stop-loss: % must be above 0 and at most 99.");
  expect(p(x.withTakeProfits(s, "", [{ pct: 2, size: 0.6 }, { pct: 4, size: 0.5 }])))
    .toContain("Take-profits: sizes must add up to less than 100%; the rest exits by the other rules.");
  expect(p(x.withTakeProfits(s, "", [{ pct: 2, size: 1 }]))).toContain("Take-profit 1: size must be above 0% and below 100% of the initial position.");
  expect(p(setAt(s, "exit.breakeven", { after: { pct: 2 }, plus: { pct: 2 } }))).toContain('Breakeven: "plus" must be below "after".');
  expect(p(setAt(s, "entry.valid_bars", 3))).toContain("Order duration applies to limit and stop orders only.");
  expect(p(setAt(x.withEntryOrder(s, "", "stop"), "entry.valid_bars", 30))).toContain("Order duration must be a whole number of bars from 1 to 20.");
  expect(p(setExecution(x.withEntryOrder(s, "", "limit"), { fill: "close" }))).toContain("Remove the entry fill: limit and stop orders fill on the bars after the signal.");
  expect(p(setExecution(s, { fill: "close", delay_bars: 2 }))).toContain(COPY.delayOnlyNextOpen);
  const si = setAt(s, "entry.scale_in", { legs: [0.6, 0.6] });
  expect(p(si)).toContain("Scale-in: use 2 to 5 legs above 0% that add up to at most 100%.");
  expect(p(x.withTakeProfits(setAt(s, "entry.scale_in", { legs: [0.5, 0.5] }), "", [{ pct: 2, size: 0.5 }]))).toContain(COPY.scaleInWithTp);
  expect(p(x.withSizingKind(s, "risk_pct"))).toContain(COPY.riskNeedsStop);
  expect(p(x.withLevel(x.withSizingKind(s, "risk_pct"), "", "exit.stop_loss", { pct: 3 }))).toEqual([]);
  expect(p(setExecution(s, { sizing: { units: 10 } }))).toContain(COPY.needsMoney);
  expect(p(setExecution(s, { sizing: { pct_equity: 150 } }))).toContain("% of equity must be above 0 and at most 100.");
  const money = x.withMode(s, "money");
  expect(p(money, idx)).toContain(MONEY_BLOCKED.index);
  expect(p(x.withDirection(money, "short"))).toContain(COPY.borrowRequired);
  expect(p(setAt(x.withDirection(money, "short"), "execution.costs.borrow_pct_year", 0))).toEqual([]);
  expect(p(setExecution(money, { cost_bps: 5 }))).toContain("Money mode takes its costs from the money settings; remove the spec's fee (cost_bps).");
  expect(p(setExecution(s, { costs: { slippage: { bps: 5 } } }))).toContain("Money costs apply in money mode only.");
  expect(p(setAt(money, "execution.costs.taxes", Array(7).fill({ name: "STT", pct: 0.1, side: "both" })))).toContain("Use at most 6 taxes.");
  expect(p(setAt(money, "execution.costs.taxes", [{ name: "STT; DROP", pct: 0.1, side: "both" }]))).toContain("Tax 1: the name must be 1 to 40 letters, digits, spaces, _ or -.");
  expect(p(setAt(money, "execution.costs.commission", { pct: 11 }))).toContain("Commission %: must be from 0 to 10.");
  expect(p(setExecution(s, { cooldown: { bars: 0 } }))).toContain("Cooldown must be a whole number of bars from 1 to 1000.");
  expect(p(setExecution(s, { max_trades: { count: 2, per: "D" as never } }))).toContain("Max entries: choose week, month, quarter or year.");
  expect(p(setExecution(s, { risk: { max_consecutive_losses: 3 } }))).toContain("Loss-streak pause needs both the number of losses and the pause in bars.");
  expect(p(setExecution(s, { risk: { max_drawdown_pct: 100 } }))).toContain("Drawdown stop must be above 0% and below 100%.");
  expect(p(setExecution(s, { instrument: { kind: "futures" } }))).toContain(FUTURES_NOTE);
});

it("edits preserve origin, unknown keys and input objects", () => {
  const s = setAt(setAt(defaultSpec(), "origin", { book: "stc9", method: "f3a", template_version: 1 }), "future_setting", { keep: 1 });
  const before = JSON.stringify(s);
  const next = x.withMode(x.withEntryOrder(x.withDirection(s, "both"), "short_side.", "stop"), "money");
  expect(next.origin).toEqual(s.origin);
  expect(next.future_setting).toEqual({ keep: 1 });
  expect(JSON.stringify(s)).toBe(before);
  const again = x.withDirection(defaultSpec(), "both");
  again.short_side!.entry.all.push(defaultSpec().entry.all[0]);
  expect(x.DEFAULT_SHORT_SIDE.entry.all).toHaveLength(1);
});

it("money capability requires explicit quantity sizing support", () => {
  const money = x.withMode(defaultSpec(), "money");
  expect(x.supportsMoney(unitCat)).toBe(false);
  expect(x.supportsMoney({ ...unitCat, execution: undefined })).toBe(false);
  expect(x.supportsMoney(cat)).toBe(true);
  expect(x.executionProblems(money, unitCat, usEtf)).toContain("Money mode is not supported by this engine yet.");
});

it("money validation mirrors current costs and sizing limits", () => {
  const money = x.withMode(defaultSpec(), "money");
  const p = (s: Spec) => x.executionProblems(s, cat, usEtf);
  expect(p(setAt(money, "execution.costs.taxes", [{ name: "STT_2024-tax", pct: 10, side: "both" }]))).toEqual([]);
  expect(p(setAt(money, "execution.costs.taxes", [{ name: "123", pct: 10, side: "buy" }]))).toEqual([]);
  expect(p(setAt(money, "execution.costs.taxes", [{ name: "STT", pct: 10.1, side: "both" }]))).toContain("Tax 1: % must be from 0 to 10.");
  expect(p(setAt(money, "execution.costs.taxes", [{ name: "STT", pct: 0, side: "oops" }]))).toContain("Tax 1: choose buy, sell or both.");
  expect(p(setExecution(money, { sizing: { units: 1.5 } }))).toContain("Shares must be a whole number from 1 to 1,000,000,000.");
  expect(p(setExecution(money, { sizing: { units: 1000000001 } }))).toContain("Shares must be a whole number from 1 to 1,000,000,000.");
  expect(p(setExecution(money, { sizing: { value: Infinity } }))).toContain("Position value must be above 0 and at most 1,000,000,000,000.");
  expect(p(setAt(money, "execution.costs.slippage", { ticks: 1 }))).toContain("Slippage in ticks needs admitted futures contract data.");
  expect(p(setExecution(money, { sizing: { units: 10, value: 1000 } as never }))).toContain("Position size: choose exactly one sizing method.");
});

it("breakeven compares ATR only when lengths match and delay checks both fills", () => {
  const p = (s: Spec) => x.executionProblems(s, cat, usEtf);
  expect(p(setAt(defaultSpec(), "exit.breakeven", { after: { atr: 2 }, plus: { atr: 2, atr_length: 14 } }))).toContain('Breakeven: "plus" must be below "after".');
  expect(p(setAt(defaultSpec(), "exit.breakeven", { after: { atr: 2 }, plus: { atr: 3, atr_length: 10 } }))).toEqual([]);
  expect(p(setExecution(defaultSpec(), { exit_fill: "close", delay_bars: 1 }))).toContain(COPY.delayOnlyNextOpen);
});

it("money risk and cooldown notes disclose their unit-return basis", () => {
  const s = setExecution(x.withMode(defaultSpec(), "money"), { cooldown: { bars: 2, after: "loss" } });
  expect(x.instrumentNotes(usEtf, s)).toContain("Risk pauses and cooldowns use the simulator's unit returns, before money rounding and declared costs.");
});

it("level edits preserve unknown metadata and do not retain an old distance form", () => {
  const s = setAt(defaultSpec(), "exit.trailing", { pct: 5, activate: { pct: 2 }, future_note: "keep" });
  const before = JSON.stringify(s);
  const next = x.withLevel(s, "", "exit.trailing", { atr: 2 });
  expect(next.exit.trailing).toEqual({ atr: 2, activate: { pct: 2 }, future_note: "keep" });
  expect(JSON.stringify(s)).toBe(before);
  const sizing = setExecution(defaultSpec(), { sizing: { pct_equity: 10, future_note: "keep" } as never });
  expect(x.withSizingKind(sizing, "risk_pct").execution?.sizing).toEqual({ risk_pct: 1, future_note: "keep" });
});

it("malformed saved lists return problems rather than crashing", () => {
  for (const tps of [{ pct: 2, size: 0.5 }, [null]]) {
    const s = setAt(defaultSpec(), "exit.take_profits", tps);
    expect(() => x.executionProblems(s, cat, usEtf)).not.toThrow();
    expect(x.executionProblems(s, cat, usEtf).length).toBeGreaterThan(0);
  }
  for (const taxes of [{ name: "STT" }, [null], [{ name: 123, pct: 0.1, side: "buy" }]]) {
    const s = setAt(x.withMode(defaultSpec(), "money"), "execution.costs.taxes", taxes);
    expect(() => x.executionProblems(s, cat, usEtf)).not.toThrow();
    expect(x.executionProblems(s, cat, usEtf).length).toBeGreaterThan(0);
  }
});

it("shared timing values and engine-supplied risk caps are enforced", () => {
  const cap = { ...cat, execution: { ...cat.execution, max_cooldown_bars: 5, max_pause_bars: 20 } };
  const p = (s: Spec) => x.executionProblems(s, cap, usEtf);
  expect(p(setExecution(defaultSpec(), { cooldown: { bars: 6 } }))).toContain("Cooldown must be a whole number of bars from 1 to 5.");
  expect(p(setExecution(defaultSpec(), { risk: { max_consecutive_losses: 1, pause_bars: 21 } }))).toContain("Loss-streak pause: losses 1 to 1000, pause 1 to 20 bars.");
  for (const key of ["fill", "exit_fill", "entry_on", "intrabar"])
    expect(p(setAt(defaultSpec(), `execution.${key}`, "unknown")).length).toBeGreaterThan(0);
});

it("malformed saved sizing and slippage stay inspectable and are refused", () => {
  const money = x.withMode(defaultSpec(), "money");
  for (const [path, value] of [["execution.sizing", "future"], ["execution.costs.slippage", "future"]] as const) {
    const s = setAt(money, path, value);
    expect(() => x.executionProblems(s, cat, usEtf)).not.toThrow();
    expect(x.executionProblems(s, cat, usEtf).length).toBeGreaterThan(0);
  }
});

it("money limit reversals are refused until their originating order kind is retained", () => {
  const both = x.withMode(x.withEntryOrder(x.withDirection(defaultSpec(), "both"), "short_side.", "limit"), "money");
  const msg = "Money mode cannot classify a resting limit reversal's fill costs; use unit mode or opposite Exit/Ignore.";
  expect(x.executionProblems(both, cat, usEtf)).toContain(msg);
  expect(x.executionProblems(setExecution(both, { opposite: "exit" }), cat, usEtf)).not.toContain(msg);
  expect(x.executionProblems(x.withMode(both, "unit"), cat, usEtf)).not.toContain(msg);
});

it("trailing % of profit: default, form swap, and the engine's bounds", () => {
  expect(x.defaultLevel("exit.trailing", "profit_pct", false)).toEqual({ profit_pct: 40 });
  const swapped = x.withLevel(setAt(defaultSpec(), "exit.trailing", { pct: 5, activate: { pct: 2 } }), "", "exit.trailing", { profit_pct: 40 });
  expect(swapped.exit.trailing).toEqual({ activate: { pct: 2 }, profit_pct: 40 }); // no stale pct (the engine refuses it)
  const problems = (slot: string, v: number) => x.executionProblems(setAt(defaultSpec(), slot, { profit_pct: v }), cat, usEtf);
  expect(problems("exit.trailing", 40)).toEqual([]);
  expect(problems("exit.trailing", 0)).toContain("Trailing stop: % of profit must be above 0 and below 100.");
  expect(problems("exit.trailing", 100)).toContain("Trailing stop: % of profit must be above 0 and below 100.");
  expect(problems("exit.stop_loss", 40)).toContain("Stop-loss: % of profit works only for the trailing stop.");
});
