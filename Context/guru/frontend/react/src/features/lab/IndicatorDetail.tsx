import { useEffect, useRef } from "react";
import { COPY } from "./labels";
import type { Catalogue, Indicator, Needs } from "./types";
import "./library.css";

const NEEDS: Record<keyof Needs, string> = {
  volume: "volume", intraday: "intraday (minute) bars", breadth: "market breadth data", second_symbol: "a second symbol",
  fundamentals: "fundamentals", derivatives: "derivatives data", onchain: "on-chain data",
};

export function IndicatorDetail({ indicator: ind, catalogue, onClose }: { indicator: Indicator; catalogue: Catalogue; onClose(): void }) {
  const needs = (Object.keys(NEEDS) as (keyof Needs)[]).filter((k) => ind.needs[k]).map((k) => NEEDS[k]);
  const box = useRef<HTMLElement>(null);
  // It renders under the chart: bring it into view when a library "i" opens it.
  useEffect(() => { box.current?.scrollIntoView?.({ block: "nearest" }); }, [ind.id]);
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
      {ind.computed ? (
        <p>
          Compare any output with a number, a price ({catalogue.price_sources.join(", ")}) or another indicator,
          on {catalogue.timeframes.join(", ")} bars. {COPY.wmRule}
        </p>
      ) : (
        <p>{ind.note} Rules cannot reference it until that data exists.</p>
      )}
      </div>
      </div>
    </section>
  );
}
