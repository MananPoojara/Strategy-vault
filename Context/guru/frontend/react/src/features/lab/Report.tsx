import { useState, type ReactNode } from "react";
import { DASH, int, num, pct } from "./format";
import { nBarStop } from "./AmiSettings";
import { directionOf, SIDE_LABEL, TRADE_SIDES, tradeKeys, usesTradePrices } from "./execution-model";
import { exprKey, nodeKind } from "./rule-tree";
import { ENTRY_KIND_LABELS } from "./labels";
import type { LabReport, MoneySummary, ReportGroup, ReportSide, RunResult, Spec, StopKey, Trade } from "./types";
import { Segmented } from "./ui";
import "./report.css";

/** What the report needs from the run beyond the report itself (optional: the report still renders without it). */
export type ReportRun = Pick<RunResult, "from" | "to"> & { dates?: string[]; instrument?: Pick<RunResult["instrument"], "ticker">; money?: Pick<MoneySummary, "capital" | "final_equity" | "costs_total"> };

type Col = "all" | "long" | "short" | "bh";
const COLS: [Col, string][] = [["all", "All trades"], ["long", "Long trades"], ["short", "Short trades"], ["bh", "Buy&Hold"]];
type Line = [label: string, cell: (c: Col) => string];
type Block = { key: string; head?: Line; rows: Line[] };

/** Trade-based rows have no buy-and-hold value. */
const NO_BH = new Set(["max_trade_dd", "sharpe_trades"]);

/**
 * One AmiBroker statistics table (All | Long | Short | Buy&Hold): the capital block, trades, winners, losers, then the
 * system rows. The engine computes system metrics on the whole equity line only, so Long/Short show "—" there.
 */
export function reportBlocks(report: LabReport, run?: ReportRun): Block[] {
  const bh = report.buy_hold, money = run?.money, start = money?.capital ?? 1, cash = (v: number | null | undefined) => num(v, money ? 2 : 3);
  const sys = (key: string, fmt: (v: number | null | undefined) => string): Line[1] => (c) =>
    c === "all" ? fmt(report.system[key]) : c === "bh" && bh && !NO_BH.has(key) ? fmt(bh[key]) : DASH;
  // Buy&Hold per-side cells come from report.buyhold (one buy-and-hold trade); older responses have none.
  const side = (fmt: (s: ReportSide) => string): Line[1] => (c) => { const s = c === "bh" ? report.buyhold : report[c]; return s ? fmt(s) : DASH; };
  const group = (g: "winners" | "losers", fmt: (x: ReportGroup) => string) => side((s) => (s[g] ? fmt(s[g]) : DASH));
  const net = (c: Col) => (c === "all" ? (money ? money.final_equity - start : typeof report.system.net_profit === "number" ? start * report.system.net_profit : null)
    : c === "bh" && typeof bh?.net_profit === "number" ? start * bh.net_profit : null);
  const p2 = (v: unknown) => pct(v, 2);
  const groupRows = (g: "winners" | "losers", word: string, largest: string): Block => ({
    key: g, head: [g === "winners" ? "Winners" : "Losers", group(g, (x) => `${int(x.count)} (${pct(x.pct, 2)})`)],
    rows: [[`Total ${word} (× initial equity)`, group(g, (x) => num(x.profit, 3))], ["Sum of trade %", group(g, (x) => pct(x.total, 1))],
      [`Avg. ${word} %`, group(g, (x) => p2(x.avg))], ["Avg. Bars Held", group(g, (x) => num(x.avg_bars, 1))], ["Max. Consecutive", group(g, (x) => int(x.max_consecutive))],
      [`Largest ${largest} %`, group(g, (x) => p2(x.largest))], [`# bars in largest ${largest}`, group(g, (x) => int(x.bars_in_largest))]],
  });
  return [
    { key: "capital", rows: [
      ["Initial capital", () => cash(start)],
      ["Ending capital", (c) => { const n = net(c); return n === null ? DASH : cash(start + n); }],
      ["Net Profit", (c) => cash(net(c))],
      ["Net Profit %", sys("net_profit", p2)], ["Exposure %", sys("exposure", p2)], ["Net Risk Adjusted Return %", sys("net_rar", p2)],
      ["Annual Return %", sys("cagr", p2)], ["Risk Adjusted Return %", sys("rar", p2)],
      ["Transaction costs", (c) => (c === "all" && money ? cash(money.costs_total) : DASH)],
    ] },
    { key: "trades", head: ["All trades", side((s) => int(s.trades))], rows: [["Avg. Profit/Loss %", side((s) => p2(s.avg))], ["Avg. Bars Held", side((s) => num(s.avg_bars, 1))]] },
    groupRows("winners", "Profit", "win"),
    groupRows("losers", "Loss", "loss"),
    { key: "system", rows: [
      ["Max. trade % drawdown", sys("max_trade_dd", p2)], ["Max. system drawdown (× initial equity)", sys("max_sys_dd_abs", (v) => num(v, 3))],
      ["Max. system % drawdown", sys("max_sys_dd", p2)], ["Recovery Factor", sys("recovery_factor", num)], ["CAR/MaxDD", sys("car_mdd", num)],
      ["RAR/MaxDD", sys("rar_mdd", num)], ["Profit Factor", side((s) => num(s.profit_factor))], ["Payoff Ratio", side((s) => num(s.payoff))],
      ["Standard Error (× initial equity)", sys("std_error", (v) => num(v, 3))], ["Risk-Reward Ratio", sys("rrr", num)], ["Ulcer Index", sys("ulcer_index", num)],
      ["Ulcer Performance Index", sys("upi", num)], ["Sharpe Ratio of trades", sys("sharpe_trades", num)], ["K-Ratio", sys("k_ratio", (v) => num(v, 3))],
    ] },
  ];
}

const dist = (d: unknown): string => {
  if (!d || typeof d !== "object") return String(d);
  const v = d as Record<string, unknown>;
  const base = "profit_pct" in v ? `${v.profit_pct}% of profit` : "pct" in v ? `${v.pct}%` : "atr" in v ? `${v.atr} × ATR(${v.atr_length ?? 14})` : "points" in v ? `${v.points} points` : "at" in v ? "at a level" : JSON.stringify(v);
  return "activate" in v ? `${base}, from ${dist(v.activate)}` : base;
};
const POSITIONS = { long: "Long", short: "Short", both: "Long and short" } as const;
const SAME_BAR: Record<string, string> = { stop_first: "Stop fills first", ohlc_path: "Open-high-low-close path" };
const REENTRY: Record<string, string> = { stop_loss: "after a stop-loss", target: "after a profit target", trailing: "after a trailing stop", bars: "after an N-bar stop" };
const SIZING: Record<string, (v: unknown) => string> = { pct_equity: (v) => `${v}% of equity`, risk_pct: (v) => `risk ${v}% of equity`, units: (v) => `${v} units`, value: (v) => `${Number(v).toLocaleString("en-US")} per trade` };
const bars = (n: number) => `${n} bar${n === 1 ? "" : "s"}`;
const kind = (k: string) => ENTRY_KIND_LABELS[k] ?? k;
const cap = (k: string) => k[0].toUpperCase() + k.slice(1);

/** The date filter that ran (none when absent, switched off or empty). */
const activeDateFilter = (spec: Spec) => (spec.date_filter && !spec.date_filter.off && spec.date_filter.windows.length ? spec.date_filter : undefined);

/** AmiBroker report "Settings" page: what produced the result, from the spec that ran, in readable words. */
export function settingsLines(spec: Spec, run?: ReportRun): [string, string][] {
  const ex = spec.execution ?? {}, out: [string, string][] = [];
  if (run) out.push(["Range", `${run.from} → ${run.to}`]);
  out.push(["Periodicity", "Daily"], ["Positions", POSITIONS[directionOf(spec)]]);
  if (ex.capital !== undefined) out.push(["Initial equity", ex.capital.toLocaleString("en-US")]);
  if (usesTradePrices(ex)) for (const k of tradeKeys(spec)) out.push([`${SIDE_LABEL[k]} price`, `${cap(ex.prices?.[k] ?? "open")}, delay ${bars(ex.delays?.[k] ?? 1)}`]);
  else {
    out.push(["Entry price", kind(ex.fill ?? "next_open")], ["Exit price", kind(ex.exit_fill ?? ex.fill ?? "next_open")]);
    if (ex.delay_bars) out.push(["Delay", bars(ex.delay_bars)]);
  }
  if (ex.intrabar) out.push(["Same-bar order", SAME_BAR[ex.intrabar] ?? ex.intrabar]);
  for (const [pre, side, name] of [["", spec, directionOf(spec) === "both" ? "Long " : ""], ["short_side.", spec.short_side, "Short "]] as const) {
    if (!side) continue;
    const e = side.exit, stops = ([["Stop-loss", e.stop_loss], ["Profit target", e.target], ["Trailing stop", e.trailing], ["Breakeven after", e.breakeven?.after]] as const).filter(([, d]) => d !== undefined).map(([l, d]) => `${l} ${dist(d)}`);
    const n = nBarStop(spec, pre);
    if (n !== undefined) stops.push(`N-bar stop ${bars(n)}`);
    if (side.entry.limit || side.entry.stop) stops.unshift(`Entry${side.entry.limit ? ` limit ${dist(side.entry.limit)}` : ""}${side.entry.stop ? ` stop ${dist(side.entry.stop)}` : ""}`);
    if (stops.length) out.push([`${name}Orders and stops`, stops.join(" · ")]);
  }
  if (ex.reentry) out.push(["Re-entry delay", Object.entries(ex.reentry).map(([k, v]) => `${bars(v as number)} ${REENTRY[k] ?? k}`).join(" · ")]);
  if (ex.sizing) out.push(["Position size", Object.entries(ex.sizing).map(([k, v]) => SIZING[k]?.(v) ?? `${k} ${v}`).join(" · ")]);
  if (ex.limits) out.push(["Position limits", Object.entries(ex.limits).map(([k, v]) => `${k.replace(/_/g, " ")} ${v}`).join(" · ")]);
  const df = activeDateFilter(spec);
  if (df) out.push(["Date filter", `${df.set?.name ? `${df.set.name}: ` : ""}trade only ${df.mode === "outside" ? "outside" : "inside"} ${df.windows.length} window${df.windows.length === 1 ? "" : "s"} (${df.windows[0][0]} → exit date ${df.windows.at(-1)![1]})${df.exit_at_change === false ? "" : "; an open trade closes when the window changes"}`]);
  if (ex.costs) {
    const c = ex.costs, fee = c.commission;
    if (fee) out.push(["Commission", [fee.per_order && `${fee.per_order} per order`, fee.per_unit && `${fee.per_unit} per share`, fee.pct && `${fee.pct}% of value`].filter(Boolean).join(" + ") || "0"]);
    if (c.slippage) out.push(["Slippage", `${c.slippage.bps} bps per side`]);
    if (c.taxes?.length) out.push(["Taxes", c.taxes.map((t) => `${t.name} ${t.pct}% (${t.side})`).join(" · ")]);
    if (c.borrow_pct_year !== undefined) out.push(["Borrow cost", `${c.borrow_pct_year}% a year`]);
  } else if (ex.cost_bps !== undefined) out.push(["Fees", `${ex.cost_bps} bps per side`]);
  return out;
}

/** Indicator settings used by the rules (AmiBroker "Parameters"), in rule order, each name once. */
export function parameterLines(spec: Spec): [string, string][] {
  const out = new Map<string, string>();
  const walk = (v: unknown): void => {
    if (Array.isArray(v)) { v.forEach(walk); return; }
    if (!v || typeof v !== "object") return;
    const o = v as Record<string, unknown>;
    if (typeof o.s === "string" && o.p && typeof o.p === "object") for (const [k, x] of Object.entries(o.p)) out.set(`${o.s} ${k}`, String(x));
    Object.values(o).forEach(walk);
  };
  walk([spec.entry, spec.exit, spec.short_side]);
  return [...out];
}

const PRICE_VAR: Record<string, string> = { open: "O", high: "H", low: "L", close: "C", volume: "V", hl2: "(H + L) / 2", hlc3: "(H + L + C) / 3", ohlc4: "(O + H + L + C) / 4", oc2: "(O + C) / 2" };
const AFL_OP: Record<string, (a: string, b: string) => string> = {
  ">": (a, b) => `${a} > ${b}`, "<": (a, b) => `${a} < ${b}`, ">=": (a, b) => `${a} >= ${b}`, "<=": (a, b) => `${a} <= ${b}`, "=": (a, b) => `${a} == ${b}`, "!=": (a, b) => `${a} != ${b}`,
  crosses_above: (a, b) => `Cross(${a}, ${b})`, crosses_below: (a, b) => `Cross(${b}, ${a})`, crosses: (a, b) => `(Cross(${a}, ${b}) OR Cross(${b}, ${a}))`,
};
const AFL_WIN: Record<string, string> = { sum: "Sum", avg: "MA", highest: "HHV", lowest: "LLV" };
/** AFL-like text of one operand; `held` is the entry variable bars_held counts from (Buy or Short). */
function aflOperand(o: unknown, held: string): string {
  if (!o || typeof o !== "object") return String(o);
  const x = o as Record<string, unknown>, k = exprKey(o), op = (key: string) => aflOperand(x[key], held), cond = (key: string) => aflNode(x[key], held);
  if ("value" in x && !k) return String(x.value);
  if ("since_entry" in x) {
    const base = x.since_entry === "highest" ? `HighestSince(${held}, ${op("of")})` : x.since_entry === "lowest" ? `LowestSince(${held}, ${op("of")})`
      : x.since_entry === "count" ? `Sum(${cond("when")}, BarsSince(${held}))` : x.since_entry === "entry_price" ? `ValueWhen(${held}, ${held}Price)` : "/* kept as is */";
    const mul = typeof x.mul === "number" && x.mul !== 1 ? ` * ${x.mul}` : "", add = typeof x.add === "number" && x.add !== 0 ? ` ${x.add < 0 ? "-" : "+"} ${Math.abs(x.add)}` : "";
    return mul || add ? `(${base}${mul}${add})` : base;
  }
  if (typeof x.s === "string") {
    const p = x.p && typeof x.p === "object" ? Object.values(x.p) : [], args = [...p, ...(x.vs ? [`"${x.vs}"`] : [])];
    let v = x.s === "bars_held" ? `BarsSince(${held})` : PRICE_VAR[x.s] ?? `${x.s.toUpperCase()}${args.length ? `(${args.join(", ")})` : ""}`;
    if (x.out) v += `.${x.out}`;
    if (x.symbol) v = `Foreign("${x.symbol}", ${v})`;
    if (x.tf && x.tf !== "D") v = `TimeFrame("${x.tf}", ${v})`;
    return x.offset ? `Ref(${v}, -${x.offset})` : v;
  }
  if (!k) return "/* kept as is */";
  const pair = Array.isArray(x[k]) ? (x[k] as unknown[]).map((v) => aflOperand(v, held)) : [];
  switch (k) {
    case "add": return `(${pair.join(" + ")})`; case "sub": return `(${pair.join(" - ")})`; case "mul": return `(${pair.join(" * ")})`; case "div": return `(${pair.join(" / ")})`;
    case "min": case "max": return `${cap(k)}(${pair.join(", ")})`;
    case "pct": return `100 * (${pair[0]} - ${pair[1]}) / ${pair[1]}`;
    case "abs": return `abs(${op(k)})`;
    case "ref": return `Ref(${op(k)}, -${x.bars})`;
    case "change": return `(${op(k)} - Ref(${op(k)}, -${x.bars}))`;
    case "bars_since": return `BarsSince(${cond(k)})`;
    case "value_when": return `ValueWhen(${cond(k)}, ${op("of")})`;
    case "highest_since": return `HighestSince(${cond(k)}, ${op("of")})`;
    case "lowest_since": return `LowestSince(${cond(k)}, ${op("of")})`;
    case "count": return `Sum(${cond(k)}, ${x.bars})`;
    case "compress": return `TimeFrame("${x.tf}", ${op(k)})`;
    default: return AFL_WIN[k] ? `${AFL_WIN[k]}(${op(k)}, ${x.bars})` : "/* kept as is */";
  }
}
function aflNode(n: unknown, held: string): string {
  const x = (n ?? {}) as Record<string, unknown>, op = (key: string) => aflOperand(x[key], held);
  switch (nodeKind(n)) {
    case "cmp": return (AFL_OP[x.op as string] ?? ((a, b) => `${a} ${x.op} ${b}`))(op("lhs"), op("rhs"));
    case "range": return x.op === "between" ? `(${op("lhs")} >= ${op("lo")} AND ${op("lhs")} <= ${op("hi")})` : `(${op("lhs")} < ${op("lo")} OR ${op("lhs")} > ${op("hi")})`;
    case "trend": return `Sum(${op("lhs")} ${x.op === "rising" ? ">" : "<"} Ref(${op("lhs")}, -1), ${x.bars}) == ${x.bars}`;
    case "all": return aflList(x.all as unknown[], " AND ", held);
    case "any": return aflList(x.any as unknown[], " OR ", held);
    case "not": return `NOT ${aflNode(x.not, held)}`;
    case "at_least": return `Sum(${aflNode(x.when, held)}, ${x.of}) >= ${x.at_least}`;
    case "then": return `Then(${(x.then as unknown[]).map((c) => aflNode(c, held)).join(", ")}; within ${x.within} bars)`;
    default: return "/* kept as is */";
  }
}
/** Switched-off conditions (rules.rs `off`) are skipped, as the engine skips them. */
const aflList = (all: unknown[] | undefined, join: string, held: string) => {
  const list = all?.filter((c) => (c as { off?: unknown })?.off !== true);
  return list?.length ? `(${list.map((c) => aflNode(c, held)).join(join)})` : "0";
};
const aflDist = (d: unknown): [mode: string, amount: string] => {
  const v = (d ?? {}) as Record<string, unknown>;
  return "pct" in v ? ["stopModePercent", String(v.pct)] : "profit_pct" in v ? ["stopModeRisk", String(v.profit_pct)] : "points" in v ? ["stopModePoint", String(v.points)]
    : "atr" in v ? ["stopModePoint", `${v.atr} * ATR(${v.atr_length ?? 14})`] : ["stopModePoint", `/* ${dist(d)} */`];
};

/**
 * AmiBroker report "Formula" page: the spec's rules written in AFL-like syntax so they read like an AmiBroker formula.
 * A readable rendering only: it is not real AFL and AmiBroker would not run it as written.
 */
export function formulaText(spec: Spec): string {
  const dir = directionOf(spec), ex = spec.execution ?? {}, out = ["// Readable rendering of the lab rules in AFL-like syntax: not real AFL, not run by AmiBroker."];
  type S = [entry: string, exit: string, side: { entry: Spec["entry"]; exit: Spec["exit"] } | undefined, pre: "" | "short_side."];
  const sides: S[] = dir === "short" ? [["Short", "Cover", spec, ""]] : dir === "both" ? [["Buy", "Sell", spec, ""], ["Short", "Cover", spec.short_side, "short_side."]] : [["Buy", "Sell", spec, ""]];
  for (const [entry, exit, side, pre] of sides) {
    if (!side) continue;
    // the N-bar stop is written as ApplyStop below, not as a Sell rule
    const n = nBarStop(spec, pre), at = n === undefined ? -1 : side.exit.any.findIndex((r) => { const c = r as { lhs?: { s?: string }; op?: string; rhs?: { value?: unknown } }; return c.lhs?.s === "bars_held" && c.op === ">=" && typeof c.rhs?.value === "number"; });
    const rules = side.exit.any.filter((_, i) => i !== at);
    out.push(`${entry} = ${aflList(side.entry.all, " AND ", entry)};`, `${exit} = ${aflList(rules, " OR ", entry)};`);
    // a filter's "also sell when it turns false" (rules.rs filter_exits), as the AFLs' ST Exit / Base exit
    for (const r of side.entry.all) { const x = r as Record<string, unknown>; if (x.filter_exit === true && x.off !== true) out.push(`${exit} = ${exit} OR NOT ${aflList([r], " AND ", entry)};`); }
    const e = side.exit, tag = dir === "both" ? ` // ${entry === "Buy" ? "long" : "short"} side` : "";
    // ApplyStop's 4th argument ExitAtStop, written when it is not the default 1
    const at4 = (k: StopKey) => { const m = ex.exit_at_stop?.[k]; return m === undefined || m === 1 ? "" : `, ${m}`; };
    for (const [type, d, k] of [["stopTypeLoss", e.stop_loss, "stop_loss"], ["stopTypeProfit", e.target, "target"], ["stopTypeTrailing", e.trailing, "trailing"]] as const) if (d) out.push(`ApplyStop(${type}, ${aflDist(d).join(", ")}${at4(k)});${tag}${type === "stopTypeTrailing" && e.trailing?.activate ? ` // activates from ${dist(e.trailing.activate)}` : ""}`);
    if (n !== undefined) out.push(`ApplyStop(stopTypeNBar, stopModeBars, ${n}${at4("bars")});${tag}${ex.nbar_has_priority ? " // N-bar stop has priority" : ""}`);
    const dual = e.dual_atr as { initial: { period: number; mult: number; price?: string }; favour: { period: number; mult: number; price?: string }; tf?: string; period_end_only?: boolean } | undefined;
    if (dual) out.push(`// Dual ATR exit: initial ${PRICE_VAR[dual.initial.price ?? "hl2"]} - ${dual.initial.mult} * ATR(${dual.initial.period}), favour ${PRICE_VAR[dual.favour.price ?? "hl2"]} -/+ ${dual.favour.mult} * ATR(${dual.favour.period}), ATR = MA(TrueRange)${dual.tf && dual.tf !== "D" ? ` on ${dual.tf} bars${dual.period_end_only ? ", exit tested at the period end" : ""}` : ""}${tag}`);
    if (e.breakeven) out.push(`// Breakeven stop after ${dist(e.breakeven.after)}${e.breakeven.plus ? `, plus ${dist(e.breakeven.plus)}` : ""}${tag}`);
    if (e.take_profits?.length) out.push(`// Take-profits: ${e.take_profits.map((t) => `${t.size}% at ${dist(t)}`).join(", ")}${tag}`);
    if (side.entry.limit) out.push(`// ${entry} limit order at ${dist(side.entry.limit)}${side.entry.valid_bars ? `, valid ${bars(side.entry.valid_bars)}` : ""}`);
    if (side.entry.stop) out.push(`// ${entry} stop order at ${dist(side.entry.stop)}${side.entry.valid_bars ? `, valid ${bars(side.entry.valid_bars)}` : ""}`);
  }
  const df = activeDateFilter(spec);
  if (df) {
    // AFL DateNum(): (year - 1900) * 10000 + month * 100 + day; windows are [start, exit date)
    const dn = (iso: string) => (Number(iso.slice(0, 4)) - 1900) * 10000 + Number(iso.slice(5, 7)) * 100 + Number(iso.slice(8, 10));
    out.push(`// Date filter${df.set?.name ? ` "${df.set.name}"` : ""}: trade only ${df.mode === "outside" ? "outside" : "inside"} these windows [start, exit date)`,
      `InWindow = ${df.windows.map(([f, t]) => `(DateNum() >= ${dn(f)} AND DateNum() < ${dn(t)})`).join(" OR ")};`,
      `DateOK = ${df.mode === "outside" ? "NOT InWindow" : "InWindow"};`);
    for (const [entry, exit, side] of sides) if (side) out.push(`${entry} = ${entry} AND DateOK;`, ...(df.exit_at_change === false ? [] : [`${exit} = ${exit} OR NOT DateOK;`]));
  }
  if (usesTradePrices(ex)) {
    const keys = tradeKeys(spec);
    out.push(`SetTradeDelays(${TRADE_SIDES.map((k) => (keys.includes(k) ? ex.delays?.[k] ?? 1 : 0)).join(", ")});`, ...keys.map((k) => `${SIDE_LABEL[k]}Price = ${cap(ex.prices?.[k] ?? "open")};`));
  } else {
    const [fill, exitFill] = [ex.fill ?? "next_open", ex.exit_fill ?? ex.fill ?? "next_open"], d = (f: string) => (f === "close" ? 0 : 1 + (ex.delay_bars ?? 0));
    const [entryVar, exitVar] = dir === "short" ? ["Short", "Cover"] : ["Buy", "Sell"];
    out.push(`SetTradeDelays(${d(fill)}, ${d(exitFill)}, ${d(fill)}, ${d(exitFill)});`, `${entryVar}Price = ${fill === "close" ? "Close" : "Open"}; ${exitVar}Price = ${exitFill === "close" ? "Close" : "Open"};`);
  }
  const sz = ex.sizing as Record<string, number> | undefined;
  if (sz) for (const [k, v] of Object.entries(sz)) out.push(k === "pct_equity" ? `SetPositionSize(${v}, spsPercentOfEquity);` : k === "units" ? `SetPositionSize(${v}, spsShares);` : k === "value" ? `SetPositionSize(${v}, spsValue);` : `// Position size: ${SIZING[k]?.(v) ?? `${k} ${v}`}`);
  // Settings the lab defaults differently from AmiBroker, or that a spec changed
  out.push(`SetOption("ActivateStopsImmediately", ${ex.activate_stops_immediately === false ? "False" : "True"});`);
  if (ex.allow_same_bar_exit === false) out.push(`SetOption("AllowSameBarExit", False);`);
  if (ex.reenter_on_exit_bar) out.push("// Re-enter on the exit bar: the sell is checked before the buy on each bar, so a new trade may open on the bar one closed");
  if (ex.limits?.margin_pct !== undefined && ex.limits.margin_pct < 100) out.push(`SetOption("AccountMargin", ${ex.limits.margin_pct});`);
  if (ex.tick_size) out.push(`TickSize = ${ex.tick_size};`);
  return out.join("\n");
}

/** AmiBroker report "Symbols" page: one row per symbol the run traded (portfolio) or the single instrument with its range and bars. */
export function symbolRows(trades: Trade[], run?: ReportRun, report?: LabReport): string[][] {
  const winPct = (ts: Trade[]) => { const closed = ts.filter((t) => !t.open && typeof t.ret === "number"); return closed.length ? pct(closed.filter((t) => t.ret! > 0).length / closed.length, 1) : DASH; };
  const tickerOf = (t: Trade) => (t as { ticker?: unknown }).ticker, tickers = [...new Set(trades.flatMap((t) => { const k = tickerOf(t); return typeof k === "string" ? [k] : []; }))];
  if (!tickers.length) {
    const money = run?.money, net = money ? num(money.final_equity - money.capital, 2) : pct(report?.system.net_profit, 2);
    return [[run?.instrument?.ticker ?? "This instrument", run?.from ?? DASH, run?.to ?? DASH, run?.dates ? int(run.dates.length) : DASH, int(trades.length), net, winPct(trades)]];
  }
  return tickers.sort().map((tk) => {
    const ts = trades.filter((t) => tickerOf(t) === tk) as (Trade & { qty?: number; costs?: number })[];
    // closed trades only: (exit − entry) × qty × direction − declared costs
    const net = ts.reduce((s, t) => (t.open || t.entry_px == null || t.exit_px == null || typeof t.qty !== "number" ? s : s + (t.exit_px - t.entry_px) * t.qty * t.dir - (t.costs ?? 0)), 0);
    return [tk, ts.map((t) => t.entry).sort()[0], ts.map((t) => (t.open ? "" : t.exit)).sort().at(-1) || "open", DASH, int(ts.length), num(net, 2), winPct(ts)];
  });
}

/** Trade % returns bucketed by `step` (AmiBroker "distribution charts spacing", default 5 %). */
export function distribution(rets: number[], step = 0.05): { from: number; count: number }[] {
  if (!rets.length) return [];
  const lo = Math.floor(Math.min(...rets) / step), hi = Math.floor(Math.max(...rets) / step);
  const out = Array.from({ length: hi - lo + 1 }, (_, i) => ({ from: (lo + i) * step, count: 0 }));
  for (const r of rets) out[Math.floor(r / step) - lo].count++;
  return out;
}

/** Shared chart frame: caption, y-range labels on the left, x labels (start · title · end) under the plot. */
export function ChartFrame({ caption, sub, label, y, x, data, children }: { caption: string; sub?: string; label: string; y: [string, string]; x: [string, string, string]; data: Record<string, string>; children: ReactNode }) {
  return (
    <figure className="lab-report-chart" {...data}>
      <figcaption>{caption}{sub && <small>{sub}</small>}</figcaption>
      <div className="lab-plot">
        <div className="lab-plot-y"><span>{y[0]}</span><span>{y[1]}</span></div>
        <svg viewBox="0 0 100 60" preserveAspectRatio="none" role="img" aria-label={label}>{children}</svg>
        <span />
        <div className="lab-report-axis"><span>{x[0]}</span><span>{x[1]}</span><span>{x[2]}</span></div>
      </div>
    </figure>
  );
}

function Histogram({ values, caption, axis, data }: { values: number[]; caption: string; axis: string; data: Record<string, string> }) {
  const bins = distribution(values);
  if (!bins.length) return null;
  const max = Math.max(1, ...bins.map((b) => b.count)), w = 100 / bins.length;
  return (
    <ChartFrame caption={caption} sub="trades per 5 % bucket" label={`${caption} chart`} data={data} y={[int(max), "0"]}
      x={[pct(bins[0].from, 0), axis, pct(bins[bins.length - 1].from + 0.05, 0)]}>
      {bins.map((b, i) => <rect key={i} x={i * w + w * 0.1} width={w * 0.8} y={57 - (b.count / max) * 54} height={(b.count / max) * 54}
        className={b.from >= 0 ? "pos" : "neg"}><title>{`${pct(b.from, 0)} to ${pct(b.from + 0.05, 0)}: ${b.count} trades`}</title></rect>)}
      <line x1="0" x2="100" y1="57" y2="57" className="axis" />
    </ChartFrame>
  );
}

/** MAE (or MFE) against the trade's own return, winners green and losers red (AmiBroker MAE/MFE distribution). */
function Excursions({ trades, field, title }: { trades: Trade[]; field: "mae" | "mfe"; title: string }) {
  const pts = trades.flatMap((t) => (typeof t[field] === "number" && typeof t.ret === "number" ? [{ x: t[field] as number, y: t.ret }] : []));
  if (!pts.length) return null;
  const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y);
  const [x0, x1] = [Math.min(0, ...xs), Math.max(0, ...xs)], [y0, y1] = [Math.min(0, ...ys), Math.max(0, ...ys)];
  // inset so no point sits on the frame
  const sx = (x: number) => 6 + ((x - x0) / (x1 - x0 || 1)) * 88, sy = (y: number) => 54 - ((y - y0) / (y1 - y0 || 1)) * 48;
  return (
    <ChartFrame caption={title} sub="y: trade %" label={`${title} chart`} data={{ "data-lab-excursion": field }} y={[pct(y1, 0), pct(y0, 0)]} x={[pct(x0, 0), `${field.toUpperCase()} %`, pct(x1, 0)]}>
      <line x1={sx(0)} x2={sx(0)} y1="2" y2="58" className="axis" /><line x1="2" x2="98" y1={sy(0)} y2={sy(0)} className="axis" />
      {/* a zero-length round-capped line stays a round dot under the stretched viewBox */}
      {pts.map((p, i) => <line key={i} x1={sx(p.x)} x2={sx(p.x)} y1={sy(p.y)} y2={sy(p.y)} className={`dot ${p.y > 0 ? "pos" : "neg"}`} />)}
    </ChartFrame>
  );
}

/** AmiBroker backtest report: one statistics table, distributions and the declared conventions. */
export function ReportPanel({ report, trades, spec, run }: { report: LabReport; trades: Trade[]; spec?: Spec; run?: ReportRun }) {
  const [mode, setMode] = useState<"scatter" | "histogram">("scatter");
  const excursion = (field: "mae" | "mfe", title: string) => mode === "scatter" ? <Excursions trades={trades} field={field} title={title} />
    : <Histogram values={trades.flatMap((t) => (typeof t[field] === "number" ? [t[field] as number] : []))} caption={`${title} distribution`} axis={`${field.toUpperCase()} %`} data={{ "data-lab-excursion": field }} />;
  const params = spec ? parameterLines(spec) : [];
  const [page, setPage] = useState<"stats" | "formula" | "symbols">("stats");
  const [copied, setCopied] = useState(false);
  // the Buy&Hold column only when the engine sent buy-and-hold figures (older responses may have none)
  const cols = report.buy_hold || report.buyhold ? COLS : COLS.filter(([c]) => c !== "bh");
  const formula = spec && page === "formula" ? formulaText(spec) : "";
  const copy = () => navigator.clipboard?.writeText(formula).then(() => setCopied(true), () => undefined);
  return (
    <section className="box lab-report" data-lab-report>
      <h3 className="box-h">Report <small>AmiBroker-style statistics · simulated · not a book method</small></h3>
      <Segmented<"stats" | "formula" | "symbols"> label="Report page" value={page} onChange={(p) => { setPage(p); setCopied(false); }}
        options={[{ value: "stats", label: "Statistics" }, ...(spec ? [{ value: "formula" as const, label: "Formula" }] : []), { value: "symbols", label: "Symbols" }]} />
      {page === "formula" && spec && <div className="lab-report-formula" data-lab-report-formula>
        <p className="rnote">Readable rendering of the rules in AmiBroker AFL-like syntax: not real AFL, and not run by AmiBroker. Settings has prices, costs and limits.</p>
        <pre>{formula}</pre>
        <button type="button" className="dbtn sm" onClick={copy}>{copied ? "Copied" : "Copy formula"}</button>
      </div>}
      {page === "symbols" && <div className="tblwrap" data-lab-report-symbols>
        <table className="tbl">
          <thead><tr><th>Symbol</th><th>From</th><th>To</th><th className="n">Bars</th><th className="n">Trades</th><th className="n">Net Profit</th><th className="n">Win %</th></tr></thead>
          <tbody>{symbolRows(trades, run, report).map((r) => <tr key={r[0]}>{r.map((v, i) => <td key={i} className={i > 2 ? "n" : undefined}>{v}</td>)}</tr>)}</tbody>
        </table>
        <p className="rnote">Win % counts closed trades. {trades.some((t) => "ticker" in t) ? "Net profit per symbol: closed trades, (exit − entry) × quantity − declared costs." : run?.money ? "Net profit in money." : "Net profit as a % of initial equity."}</p>
      </div>}
      {page === "stats" && <>
      <p className="cap lab-report-constants" data-lab-report-constants>
        Risk-free rate {report.rates?.sharpe ?? 5} % (Sharpe of trades) · {report.rates?.upi ?? 5.4} % (Ulcer performance index) · distribution buckets 5 % · Buy&amp;Hold: {run?.instrument?.ticker ?? "the same instrument"}{report.buyhold ? " (trade columns: one buy-and-hold trade over the run)" : ""}
      </p>
      <div className="tblwrap">
        <table className="tbl" data-lab-report-stats>
          <thead><tr><th>Statistics</th>{cols.map(([k, label]) => <th className="n" key={k}>{k === "bh" && run?.instrument ? `${label} (${run.instrument.ticker})` : label}</th>)}</tr></thead>
          {reportBlocks(report, run).map(({ key, head, rows }) => (
            <tbody key={key} {...(key === "system" ? { "data-lab-report-system": "" } : {})}>
              {head && <tr className="lab-report-block"><td>{head[0]}</td>{cols.map(([c]) => <td className="n" key={c}>{head[1](c)}</td>)}</tr>}
              {rows.map(([label, cell]) => <tr key={label}><td>{label}</td>{cols.map(([c]) => <td className="n" key={c}>{cell(c)}</td>)}</tr>)}
            </tbody>
          ))}
        </table>
      </div>
      {trades.some((t) => typeof t.mae === "number") && <Segmented<"scatter" | "histogram"> label="MAE/MFE chart" value={mode} onChange={setMode}
        options={[{ value: "scatter", label: "MAE/MFE vs trade %" }, { value: "histogram", label: "MAE/MFE distribution" }]} />}
      <div className="lab-report-charts">
        <Histogram values={trades.flatMap((t) => (typeof t.ret === "number" ? [t.ret] : []))} caption="Profit distribution" axis="% Profit" data={{ "data-lab-profit-distribution": "" }} />
        {excursion("mae", "Max. Adverse Excursion")}
        {excursion("mfe", "Max. Favourable Excursion")}
      </div>
      {spec && <details className="lab-report-settings" data-lab-report-settings>
        <summary>Settings</summary>
        <table className="tbl"><tbody>{settingsLines(spec, run).map(([k, v]) => <tr key={k}><td className="lab-report-key">{k}</td><td>{v}</td></tr>)}</tbody></table>
        {!!params.length && <table className="tbl" data-lab-report-params><caption>Parameters</caption><tbody>{params.map(([k, v]) => <tr key={k}><td className="lab-report-key">{k}</td><td className="n">{v}</td></tr>)}</tbody></table>}
      </details>}
      <ul className="rnote lab-report-notes">{report.notes.map((n) => <li key={n}>{n}</li>)}</ul>
      </>}
    </section>
  );
}
