// What a tradesheet is about: plain exit reasons tied to the numbered rules, and a Rules sheet for the Excel download.
import { friendlyReason, ruleWords } from "./spec-model";
import { stopsOn, changedSettings } from "./defaults";
import { entryOrderType } from "./execution-model";
import type { Catalogue, RuleNode, RunResult, Spec } from "./types";

/** The test behind a result: the spec that ran (default exit included), the user's own rules, and the run settings. */
export interface RunContext { spec: Spec; own: Spec; catalogue: Catalogue; tf?: string; fees?: string | number; slippage?: string | number }

type Obj = Record<string, unknown>;
type Side = { entry: { all: RuleNode[] }; exit: { any: RuleNode[] } & Obj };
const TF: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly", Q: "Quarterly", Y: "Yearly" };
const dist = (d: unknown) => {
  const x = (d ?? {}) as Record<string, number>;
  return x.pct !== undefined ? `${x.pct}%` : x.atr !== undefined ? `${x.atr} × ATR(${x.atr_length ?? 14})` : x.points !== undefined ? `${x.points} points`
    : x.profit_pct !== undefined ? `${x.profit_pct}% of profit` : "";
};
const words = (n: RuleNode | undefined, cat: Catalogue) => (n ? ruleWords(n, cat)?.text ?? JSON.stringify(n) : "");
const sides = (s: Spec): { pre: string; side: Side; short: boolean }[] =>
  [{ pre: "", side: s as unknown as Side, short: s.direction === "short" },
    ...(s.short_side ? [{ pre: "short_side.", side: s.short_side as unknown as Side, short: true }] : [])];

/** "exit.any[2]: bars_held >= 10" → "Default exit: after 10 days"; own rows keep the number they have in the sell list. */
export function reasonLabeler(ctx: RunContext): (reason: string) => string {
  return (reason) => {
    if (reason === "open") return "Still open at the end of the test";
    const rule = /^(short_side\.)?exit\.any\[(\d+)\]/.exec(reason);
    if (rule) {
      const short = !!rule[1], i = Number(rule[2]);
      const ran = (short ? ctx.spec.short_side : ctx.spec)?.exit.any ?? [], own = (short ? ctx.own.short_side : ctx.own)?.exit.any ?? [];
      const name = short || ctx.spec.direction === "short" ? "Cover" : "Sell";
      return i >= own.length ? `Default exit: ${words(ran[i], ctx.catalogue)}` : `${name} rule ${i + 1}: ${words(ran[i], ctx.catalogue)}`;
    }
    const order = /^(short_side\.)?exit\.(stop_loss|target|trailing)( \(gap at the open\))?$/.exec(reason);
    if (order) {
      const ran = (order[1] ? ctx.spec.short_side : ctx.spec)?.exit as Obj | undefined, own = (order[1] ? ctx.own.short_side : ctx.own)?.exit as Obj | undefined;
      const name = { stop_loss: "Stop-loss", target: "Profit target", trailing: "Trailing stop" }[order[2] as "target"];
      const size = dist(ran?.[order[2]]);
      return `${own?.[order[2]] === undefined ? "Default exit: " : ""}${name}${size ? ` ${size}` : ""}${order[3] ? " (the bar opened beyond it: filled at the open)" : ""}`;
    }
    return friendlyReason(reason);
  };
}

/** The Rules sheet: what was chosen, the entry and exit conditions as numbered in the exit reasons, and how a trade works. */
export function rulesSheet(ctx: RunContext, r: RunResult): (string | number | null)[][] {
  const { spec, own, catalogue: cat } = ctx, ex = spec.execution ?? {}, dir = spec.direction ?? "long";
  const fill = ex.fill ?? "next_open", exitFill = ex.exit_fill ?? fill;
  const fillText = (f: string) => f === "close" ? "the close of the signal candle" : f === "next_open" ? "the next candle's open" : f;
  const rows: (string | number | null)[][] = [["Section", "Item", "What it is"]];
  const add = (a: string, b: string, c: string | number | null = "") => rows.push([a, b, c]);
  add("Test", "Instrument", r.instrument?.ticker ?? "");
  add("Test", "Candles", `${TF[ctx.tf ?? "D"] ?? ctx.tf} bars; rules read completed candles only (no look-ahead)`);
  add("Test", "Period", `${r.from} to ${r.to}`);
  add("Test", "Positions", dir === "short" ? "Short only" : dir === "both" ? "Long and short" : "Long only");
  add("Test", "Buy (entry) fills at", fillText(fill));
  add("Test", "Sell (exit) fills at", `${fillText(exitFill)}; stops and targets fill at their own price inside the candle`);
  add("Test", "Order type", entryOrderType(spec.entry) === "market" ? "Market order (buys at the fill price)" : `${entryOrderType(spec.entry).replace("_", "-")} order`);
  add("Test", "Costs", `fees ${Number(ctx.fees) || 0} bps + slippage ${Number(ctx.slippage) || 0} bps per side (1 bps = 0.01%)`);
  add("Test", "Results in", r.money ? "Cash amounts" : "% returns of one position");
  const changed = changedSettings(spec);
  if (changed.length) add("Test", "Other settings changed", changed.join(", "));
  for (const { pre, side, short } of sides(spec)) {
    const buy = short ? "Short" : "Buy", sell = short ? "Cover" : "Sell", tag = pre ? " (short side)" : "";
    const rowsOf = (list: RuleNode[]) => list.map((n, i) => ({ n, i, off: (n as Obj).off === true, filter: (n as Obj).filter === true }));
    const entries = rowsOf(side.entry.all);
    add(`Entry${tag}`, `${buy} when`, "ALL switched-on buy rules and filters are true on the same candle; the trade opens at the fill price above");
    for (const e of entries) add(`Entry${tag}`, `${e.filter ? "Filter" : `${buy} rule`} ${e.i + 1}${e.off ? " (switched off, not used)" : ""}`,
      words(e.n, cat) + ((e.n as Obj).filter_exit === true ? " · also sells when it turns false" : ""));
    if (!entries.some((e) => !e.off)) add(`Entry${tag}`, "No buy rule", "No trade can open");
    const ownExit = (pre ? own.short_side : own)?.exit.any ?? [];
    add(`Exit${tag}`, `${sell} when`, "ANY ONE of these is true; the first one that happens closes the trade, and its name is the Exit reason");
    side.exit.any.forEach((n, i) => {
      const off = (n as Obj).off === true, dflt = i >= ownExit.length;
      add(`Exit${tag}`, dflt ? "Default exit" : `${sell} rule ${i + 1}${off ? " (switched off, not used)" : ""}`, words(n, cat) + (dflt ? " (no sell rule was switched on, so the default exit was used)" : ""));
    });
    for (const s of stopsOn(side.exit as Obj, undefined)) add(`Exit${tag}`, "Stop / target", s); // an N-bar stop is the Bars held rule above
  }
  add("How it works", "One trade at a time", "A new trade opens only when no trade is open.");
  add("How it works", "crosses above / below", "True only on the candle where the value moves from at-or-below to above the level (or the reverse); staying above is not a new cross.");
  add("How it works", "Same-candle re-entry", fill === "close" && exitFill === "close" ? "A trade that closes at a candle's close cannot be followed by a new trade at that same close; the next signal is needed." : "An exit and a new entry may happen on the same candle when they fill at different prices.");
  add("How it works", "Counting bars", "Bars held and 'Nth up close after entry' count from the candle after the entry candle.");
  add("Columns", "Date / Price", "Entry date and fill price");
  add("Columns", "Ex. date / Ex. Price", "Exit date and fill price (“Open” if still open at the end)");
  add("Columns", "% Profit", "Return of the trade after costs");
  add("Columns", "Cum. %", "All trades so far compounded");
  add("Columns", "# bars", "Candles in the trade");
  add("Columns", "MAE / MFE", "Worst / best move against / for the trade while it was open");
  add("Columns", "Profit/bar", "% Profit ÷ (bars + 1)");
  add("Columns", "Exit reason", "Which rule closed the trade, numbered as above");
  return rows;
}
