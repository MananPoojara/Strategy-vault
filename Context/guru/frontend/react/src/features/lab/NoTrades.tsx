import { useEffect, useState } from "react";
import { neverMeets } from "./scale-check";
import { setAt } from "./spec-model";
import { tradeBlockers } from "./defaults";
import { words } from "./PlainRules";
import { int } from "./format";
import type { ScanResult } from "./client";
import type { Catalogue, RuleNode, SavedMarket, Spec } from "./types";

type Obj = Record<string, unknown>;
const MAX_ROWS = 8; // one scan per buy row; more rows are explained together only
interface Finding { all?: number; rows: { i: number; text: string; days?: number }[]; ranges: string[] }

/** A run with 0 trades, explained: how many days each buy rule is true on its own and together (raw signals, /lab/scan),
 *  rules that can never be true here, and the switched-on settings that keep a signal from becoming a trade. */
export function NoTradesHelp({ spec, catalogue, market, ticker, period, scan, onFix }: {
  spec: Spec; catalogue: Catalogue; market: SavedMarket; ticker: string; period?: { from?: string; to?: string; latest?: string };
  /** The raw signals of a spec on this instrument and period. */
  scan(spec: Spec): Promise<ScanResult>;
  /** Changes the strategy being edited and tests it again. */
  onFix(change: (spec: Spec) => Spec): void;
}) {
  const [found, setFound] = useState<Finding | null>(null);
  useEffect(() => {
    let live = true;
    setFound(null);
    const signal = spec.direction === "short" ? "Short" : "Buy";
    const days = (s: Spec) => scan(s).then((r) => r.rows.filter((x) => x[2] === signal).length, () => undefined);
    const rows = spec.entry.all.map((n, i) => ({ n, i })).filter(({ n }) => (n as Obj).off !== true);
    const alone = (n: RuleNode): Spec => { const bare = { ...(n as Obj) }; delete bare.filter; delete bare.filter_exit; return { ...spec, entry: { ...spec.entry, all: [bare as RuleNode] } }; };
    void Promise.all([days(spec), rows.length > 1 && rows.length <= MAX_ROWS ? Promise.all(rows.map(({ n }) => days(alone(n)))) : Promise.resolve([] as (number | undefined)[]),
      neverMeets(spec, catalogue, market, ticker).catch(() => [] as string[])]).then(([all, each, ranges]) => {
      if (live) setFound({ all, ranges, rows: rows.map(({ n, i }, k) => ({ i, text: words(n, catalogue).text, days: rows.length === 1 ? all : each[k] })) });
    });
    return () => { live = false; };
  }, [spec, catalogue, market, ticker, scan]);

  const off = (i: number) => <button type="button" className="dbtn sm" data-lab-fix-off={i} onClick={() => onFix((s) => setAt(s, `entry.all[${i}].off`, true))}>Test without rule {i + 1}</button>;
  if (!found) return <div className="rnote" role="status" data-lab-no-trades>No trades. Checking why…</div>;
  const blockers = tradeBlockers(spec, period), never = found.rows.filter((r) => r.days === 0), counted = found.rows.every((r) => r.days !== undefined);
  return <div className="rnote lab-no-trades" role="note" data-lab-no-trades>
    <p><b>No trades.</b> {found.all === undefined ? "Here is what can stop a trade:" : found.all === 0 ? `Your buy ${found.rows.length === 1 ? "rule was" : "rules were"} never true on ${found.rows.length === 1 ? "any day" : "the same day"} in this test.` : `Your buy rules were true on ${int(found.all)} days, but no trade opened.`}</p>
    {found.all === 0 && found.rows.length > 1 && counted && <>
      <ul className="lab-no-trades-rows">{found.rows.map((r) => <li key={r.i}>Rule {r.i + 1} · {r.text}: <b>{int(r.days)} days</b> on its own {found.rows.length > 1 && r.days ? off(r.i) : null}</li>)}
        <li>All of them together: <b>0 days</b></li></ul>
      {!never.length && <p>Each rule is true on some days, but never all on the same candle. Switch one off, or loosen a number (for example RSI below 40 instead of 30).</p>}
    </>}
    {found.rows.length === 1 && found.all === 0 && <p>Rule 1 · {found.rows[0].text}.</p>}
    {found.rows.length > 1 && never.length > 0 && <p>{never.map((r) => `Rule ${r.i + 1} (${r.text})`).join(" and ")} {never.length === 1 ? "is" : "are"} never true here.</p>}
    {found.ranges.map((n) => <p key={n}>{n}</p>)}
    {found.all !== 0 && blockers.length > 0 && <ul className="lab-no-trades-rows">{blockers.map((b) => <li key={b.id}>{b.text}{b.off && <button type="button" className="dbtn sm" onClick={() => onFix(b.off!)}>Turn off and test again</button>}</li>)}</ul>}
    {found.all !== 0 && !blockers.length && <p>Check the test period and the entry order type in Settings.</p>}
  </div>;
}
