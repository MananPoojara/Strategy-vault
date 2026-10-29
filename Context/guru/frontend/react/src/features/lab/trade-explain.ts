// What a tradesheet is about: short exit reasons named as in the Exit card, and a plain Rules sheet for the Excel download.
import { candleCount, friendlyReason, ruleWords } from "./spec-model";
import { stopsOn, changedSettings } from "./defaults";
import { describeNode, nodeKind } from "./rule-tree";
import { bandWords } from "./BandRule";
import { isSupertrendExit, nCandleOf } from "./ExitChoice";
import type { Catalogue, RuleNode, RunResult, Spec } from "./types";

/** The test behind a result: the spec that ran, the user's own rules, and the run settings. */
export interface RunContext { spec: Spec; own: Spec; catalogue: Catalogue; tf?: string; fees?: string | number; slippage?: string | number }

type Obj = Record<string, unknown>;
type Side = { entry: { all: RuleNode[] }; exit: { any: RuleNode[] } & Obj };
const TF: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly", Q: "Quarterly", Y: "Yearly" };
const bare = (n: RuleNode) => { const { off: _o, filter: _f, filter_exit: _e, ...x } = n as Obj; return x as RuleNode; };
const words = (n: RuleNode | undefined, cat: Catalogue) => (n ? bandWords(bare(n), cat) ?? ruleWords(bare(n), cat)?.text ?? describeNode(bare(n), cat) : "");
const sides = (s: Spec): { pre: string; side: Side; short: boolean }[] =>
  [{ pre: "", side: s as unknown as Side, short: s.direction === "short" },
    ...(s.short_side ? [{ pre: "short_side.", side: s.short_side as unknown as Side, short: true }] : [])];

/** An exit rule's short name, as the Exit card calls it: N candle, Supertrend, Band setup, or Exit rule N for an indicator rule. */
const exitName = (n: RuleNode | undefined, i: number) => {
  const x = n && bare(n);
  return !x ? `Exit rule ${i + 1}` : nCandleOf(x) || candleCount(x) || (x as Obj).lhs && ((x as Obj).lhs as Obj).s === "bars_held" ? "N candle"
    : isSupertrendExit(x) ? "Supertrend" : nodeKind(x) === "setup" ? "Band setup" : `Exit rule ${i + 1}`;
};
const STOPS: Record<string, string> = { stop_loss: "HSL", target: "Profit booking", trailing: "Trailing stop", breakeven: "Breakeven stop" };

/** "exit.any[0]: …" → "N candle"; "exit.stop_loss" → "HSL"; the date window, filters and the open trade in a few words. */
export function reasonLabeler(ctx: RunContext): (reason: string) => string {
  return (reason) => {
    if (reason === "open") return "Open";
    const rule = /^(short_side\.)?exit\.any\[(\d+)\]/.exec(reason);
    if (rule) return `${rule[1] ? "Short: " : ""}${exitName(((rule[1] ? ctx.spec.short_side : ctx.spec)?.exit.any ?? [])[Number(rule[2])], Number(rule[2]))}`;
    const stop = /^(short_side\.)?exit\.(stop_loss|target|trailing|breakeven)( \(gap at the open\))?$/.exec(reason);
    if (stop) return `${stop[1] ? "Short: " : ""}${STOPS[stop[2]]}${stop[3] ? " (gap)" : ""}`;
    if (/^(short_side\.)?exit\.dual_atr$/.test(reason)) return "ATR";
    if (/^(short_side\.)?exit\.dual_line$/.test(reason)) return "Supertrend";
    if (reason.startsWith("date_filter")) return "Date window ended";
    if (/entry\.all\[\d+\]: filter turned false/.test(reason)) return "Filter exit";
    return friendlyReason(reason);
  };
}

/** The Rules sheet, in the research team's layout: a title, then Item | Value under Strategy, Entry and Exit. */
export function rulesSheet(ctx: RunContext, r: RunResult): (string | number | null)[][] {
  const { spec, catalogue: cat } = ctx, dir = spec.direction ?? "long";
  const rows: (string | number | null)[][] = [["STRATEGY RULES"]];
  const head = (title: string) => rows.push([], [title]), add = (item: string, value: string) => rows.push([item, value]);
  head("Strategy");
  add("Instrument", r.instrument?.ticker ?? "");
  add("Backtest date range", `${dmy(r.from)} → ${dmy(r.to)}`);
  add("Timeframe", TF[ctx.tf ?? "D"] ?? String(ctx.tf));
  add("Positions", dir === "short" ? "Short only" : dir === "both" ? "Long and short" : "Long only");
  if (Number(ctx.fees)) add("Fees", `${Number(ctx.fees)} bps per side`);
  const changed = changedSettings(spec);
  if (changed.length) add("AmiBroker settings changed", changed.join(", "));
  // With / without filter, and which: the date periods and the indicator filter rows that were switched on
  const df = spec.date_filter, used = [
    ...(df && !df.off && df.windows.length ? [`Dates: ${df.set?.name ?? "Own dates"} · ${df.windows.length} periods`] : []),
    ...sides(spec).flatMap(({ pre, side }) => side.entry.all.filter((n) => (n as Obj).filter === true && (n as Obj).off !== true)
      .map((n) => `${pre ? "Short " : ""}Indicator: ${words(n, cat)}${(n as Obj).filter_exit === true ? " (also exits when it turns false)" : ""}`)),
  ];
  add("Filter", used.length ? `With filter — ${used.join("; ")}` : "Without filter");
  for (const { pre, side, short } of sides(spec)) {
    const tag = pre ? "Short " : "";
    head(`${tag}Entry`);
    // the filter rows are named in Strategy › Filter; Entry lists the buy rules only
    side.entry.all.filter((n) => (n as Obj).filter !== true).forEach((n, i) => add(`Entry ${i + 1}${(n as Obj).off === true ? " (off)" : ""}`, words(n, cat)));
    head(`${tag}Exit`);
    side.exit.any.forEach((n, i) => add(`${exitName(n, i)}${(n as Obj).off === true ? " (off)" : ""}`, words(n, cat)));
    for (const s of stopsOn(side.exit as Obj, undefined)) add("Stop / target", s);
  }
  return rows;
}

const dmy = (iso: string) => (/^\d{4}-\d{2}-\d{2}$/.test(iso) ? `${iso.slice(8, 10)}-${iso.slice(5, 7)}-${iso.slice(0, 4)}` : iso);
/** The research team's index codes in tradesheet tab names (NF_…, BN_…, MD_…). */
const INDEX_CODE: Record<string, string> = { "NIFTY 50": "NF", "NIFTY BANK": "BN", NIFTYBANK: "BN", "NIFTY MIDCAP 100": "MD", NIFTYMIDCAP100: "MD" };
const SHORT_IND: Record<string, string> = { supertrend: "ST", bb: "BB", bbpct: "%B", rsi: "RSI", ema: "EMA", sma: "SMA", atr: "ATR", macd: "MACD", roc: "ROC" };
/** A rule in a few characters for a tab name: the first indicator and its numbers ("ST 5,1"), or the exit's short name. */
function shortRule(n: RuleNode | undefined, exit: boolean, i: number): string {
  if (!n) return "";
  if (exit) {
    const nc = nCandleOf(bare(n));
    if (nc) return `Ncandle-${nc.n}${nc.tf === "D" ? "" : ` ${nc.tf}`}`;
    // other candle counts (green / red candles since entry): the count is the rule's number
    if (exitName(n, i) === "N candle") { const v = JSON.stringify(n).match(/"value":(\d+)/); return `Ncandle-${v?.[1] ?? ""}`; }
  }
  let found: Obj | undefined;
  const visit = (x: unknown) => { if (found || !x || typeof x !== "object") return; const o = x as Obj; if (typeof o.s === "string" && o.p && typeof o.p === "object") { found = o; return; } Object.values(o).forEach(visit); };
  visit(n);
  if (!found) return exit ? exitName(n, i) : "Price";
  const p = Object.values(found.p as Record<string, number>).slice(0, 2).join(",");
  return `${SHORT_IND[found.s as string] ?? String(found.s).toUpperCase()}${p ? ` ${p}` : ""}`;
}
/** The trade tab's name, as the team names theirs: index code, entry and exit ("NF_En_ST 5,1_EX_ST 5,1"); ts-workbook trims it to Excel's 31. */
export function tsTabName(ctx: RunContext, r: RunResult): string {
  const ticker = r.instrument?.ticker ?? "", code = INDEX_CODE[ticker.toUpperCase()] ?? ticker.replace(/\s+/g, "").slice(0, 6);
  const own = (ctx.spec.entry.all ?? []).filter((n) => (n as Obj).filter !== true && (n as Obj).off !== true), ex = (ctx.spec.exit.any ?? []).filter((n) => (n as Obj).off !== true);
  return `${code}_En_${shortRule(own[0], false, 0)}_EX_${shortRule(ex[0], true, 0)}`;
}
/** The template's Rules sheet fields, worded as in the Excel Rules list. */
export function tsRules(ctx: RunContext, r: RunResult) {
  const rows = rulesSheet(ctx, r), get = (k: string) => rows.find((x) => x[0] === k)?.[1] as string | undefined;
  const span = `${r.instrument?.ticker ?? ""} (${dmy(r.from)} to ${dmy(r.to)})`, dir = ctx.spec.direction ?? "long";
  const entry = sides(ctx.spec).flatMap(({ side }) => side.entry.all.filter((n) => (n as Obj).filter !== true && (n as Obj).off !== true).map((n) => words(n, ctx.catalogue)));
  const exit = sides(ctx.spec).flatMap(({ side }) => [...side.exit.any.filter((n) => (n as Obj).off !== true).map((n, i) => `${exitName(n, i)}: ${words(n, ctx.catalogue)}`),
    ...stopsOn(side.exit as Obj, undefined)]);
  // the last trade may still be open: the tradesheet values it at the last close, and says so here
  const open = r.trades?.find((t) => t.open);
  if (open) exit.push(`Last trade still open: valued at the close on ${dmy(open.exit || r.to)}`);
  const setup = sides(ctx.spec).flatMap(({ side }) => [...side.entry.all, ...side.exit.any]).filter((n) => nodeKind(bare(n)) === "setup").map((n) => words(n, ctx.catalogue));
  return { timeframe: TF[ctx.tf ?? "D"] ?? String(ctx.tf), index: span, period: `${dmy(r.from)} to ${dmy(r.to)}`, strategy: tsTabName(ctx, r),
    position: dir === "short" ? "Short" : dir === "both" ? "Long & Short" : "Long", filter: get("Filter") ?? "Without filter", setup: setup.join("; ") || "—", entry, exit };
}
