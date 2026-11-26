import type { ReactNode } from "react";
import { NumField } from "../OperandEditor";
import { setExecution } from "../spec-model";
import type { InstrumentContext } from "../execution-model";
import type { Catalogue, PositionLimits, Spec } from "../types";
import { cashWhy, Why } from "./common";

/** Strategy settings › Portfolio, the first card in both views (user, 26-11-2026): Backtest on and, for a portfolio, its Watchlist /
 *  Ranking / Positions tabs (PortfolioPanel, whose Positions tab holds the portfolio-level settings); one instrument shows its own settings here. */
export function PortfolioTab({ spec, catalogue, instrument, onChange, portfolio }: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void; portfolio?: ReactNode }) {
  return <section className="lab-portfolio-card lab-ami" data-lab-portfolio-card aria-label="Portfolio">
    <h3 className="lab-portfolio-card-h">Portfolio <span className="cap">{spec.portfolio ? "a watchlist sharing one account" : "the instrument on the chart"}</span></h3>
    <div className="lab-portfolio-card-body">
      {portfolio}
      {(!spec.portfolio || !portfolio) && <PortfolioLevel spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} />}
    </div>
  </section>;
}

/** One instrument's own settings (AmiBroker's Portfolio tab, volume limit); a portfolio has none here: Maximum position and
 *  previous-bar equity are in Backtest parameters. */
export function PortfolioLevel({ spec, catalogue, instrument = { kind: "index", market: "in" }, onChange }: { spec: Spec; catalogue: Catalogue; instrument?: InstrumentContext; onChange(spec: Spec): void }) {
  if (spec.portfolio) return null;
  const limits = spec.execution?.limits ?? {};
  // the volume limit is an execution.limit for money mode on one instrument (the portfolio engine refuses it)
  const limitsWhy = cashWhy(spec, catalogue, instrument);
  const setLimit = (v: number | undefined) => {
    const next: PositionLimits = { ...limits, max_volume_pct: v };
    if (v === undefined) delete next.max_volume_pct;
    onChange(setExecution(spec, { limits: Object.keys(next).length ? next : undefined }));
  };
  return <fieldset className="lab-fieldset" data-lab-portfolio-tab>
    <legend>Single-instrument settings</legend>
    <div className="lab-row">
      {/* AmiBroker's 0 means no limit, so 0 or empty removes the key */}
      <NumField label="Limit trade size as % of entry bar volume (zero means no-limit)" value={limitsWhy ? undefined : limits.max_volume_pct} min={0} max={100} placeholder="0" disabled={!!limitsWhy}
        onChange={(n) => setLimit(n || undefined)} />
      {/* AmiBroker's default, and the engine's only behaviour (ledger.rs: a bar with no volume does not cap the size) */}
      <label className="lab-check" title="Always on in the lab: a bar with zero or missing volume does not limit the trade size.">
        <input type="checkbox" aria-label="Disable trade size limit when bar volume is zero" checked disabled readOnly /> Disable trade size limit when bar volume is zero <span className="cap">(always on in the lab)</span></label>
      <Why text={limitsWhy} />
    </div>
    {!limitsWhy && !!limits.max_volume_pct && <p className="cap">The volume limit reads the last complete bar when the order fills: the fill bar for a close fill, otherwise the bar before it, so it never looks ahead.</p>}
  </fieldset>;
}
