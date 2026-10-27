import { useEffect, useRef } from "react";
import { COPY } from "./labels";
import { ORDER_FLOW_NOTE } from "./catalogue";
import { firstRule, ownExit } from "./defaults";
import { words } from "./PlainRules";
import type { Catalogue, Indicator, Needs, RuleNode } from "./types";
import "./library.css";

const NEEDS: Record<keyof Needs, string> = {
  volume: "volume", intraday: "intraday (minute) bars", breadth: "market breadth data", second_symbol: "a second symbol",
  fundamentals: "fundamentals", derivatives: "derivatives data", onchain: "on-chain data", order_flow: "order-flow data (trades at bid/ask)", order_book: "order-book (depth) data",
};

export function IndicatorDetail({ indicator: ind, catalogue, onClose, onUse }: {
  indicator: Indicator; catalogue: Catalogue; onClose(): void;
  /** Make this indicator's default rules the strategy (its buy rule, and its own exit or the default exit). */
  onUse?(entry: RuleNode, exit?: RuleNode): void;
}) {
  const needs = (Object.keys(NEEDS) as (keyof Needs)[]).filter((k) => ind.needs[k]).map((k) => NEEDS[k]);
  const box = useRef<HTMLElement>(null);
  // Inline callers scroll to the reference; a floating window keeps the chart in place.
  useEffect(() => { if (!box.current?.closest('[role="dialog"]')) box.current?.scrollIntoView?.({ block: "nearest" }); }, [ind.id]);
  return (
    <section className="box lab-detail" ref={box}>
      <h2 className="box-h">{ind.name} <small>id {ind.id} · {ind.category}</small>
        <span className="box-tools"><button type="button" className="dbtn ghost sm" aria-label="Close reference" onClick={onClose}>×</button></span>
      </h2>
      <div className="lab-detail-cols">
      <div>
      <h3>Definition</h3>
      <pre>{ind.definition}</pre>
      <dl>
        <dt>Placement</dt><dd>{ind.placement === "price" ? "On price" : "Own pane"}</dd>
        <dt>Needs</dt><dd>{needs.length ? needs.join(", ") : "Price only"}</dd>
        <dt>Warm-up bars</dt><dd>{ind.first.map((f) => f ?? "–").join(", ")}{ind.sparse ? " + confirmation" : ""}</dd>
        <dt>Timeframes</dt><dd>{catalogue.timeframes.join(" · ")}</dd>
        <dt>Engine</dt><dd>{ind.computed ? "Computed" : "Listed only"}</dd>
      </dl>
      </div>
      <div>
      {ind.params.length > 0 && (
        <div className="tblwrap">
          <table className="tbl">
            <thead><tr><th>Setting</th><th>Key</th><th>Default</th><th>Min</th><th>Max</th><th>Step</th></tr></thead>
            <tbody>
              {ind.params.map((p) => (
                <tr key={p.key}><td>{p.label}</td><td>{p.key}</td><td>{p.default}</td><td>{p.min}</td><td>{p.max}</td><td>{p.step ?? "–"}</td></tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <h3>Outputs</h3>
      <ul>{ind.outputs.map((o) => <li key={o}>{o}</li>)}</ul>
      <p>{COPY.notChecked}</p>
      <h3>Using it in rules</h3>
      {ind.computed ? (<>
        <p>
          Compare any output with a number, a price ({catalogue.price_sources.join(", ")}) or another indicator,
          on {catalogue.timeframes.join(", ")} bars. {COPY.wmRule} {ind.needs.order_flow && ORDER_FLOW_NOTE}
        </p>
        <DefaultRules ind={ind} catalogue={catalogue} onUse={onUse} />
      </>) : (
        <p>{ind.note} Rules cannot reference it until that data exists.</p>
      )}
      </div>
      </div>
    </section>
  );
}

/** The buy rule a pick adds and its own exit (TradingView style), with the default settings; "Use" makes them the strategy. */
function DefaultRules({ ind, catalogue, onUse }: { ind: Indicator; catalogue: Catalogue; onUse?(entry: RuleNode, exit?: RuleNode): void }) {
  const entry = firstRule(ind, {}), exit = ownExit(ind, {}), sell = exit && words(exit, catalogue);
  return <div className="lab-default-rules" data-lab-default-rules>
    <h3>Default rules (TradingView style)</h3>
    <p><b>Buy</b> when {words(entry, catalogue).text}. <b>Sell</b> {sell ? `${sell.when ? "when " : ""}${sell.text}` : "with the default exit: the 1st up close after entry, or the 4th down close"}.</p>
    {onUse && <button type="button" className="dbtn sm primary" data-lab-use-rules onClick={() => onUse(entry, exit)}>Use as my rules</button>}
    {onUse && <p className="cap">Replaces your buy and sell rules; every number can be changed in Your idea.</p>}
  </div>;
}
