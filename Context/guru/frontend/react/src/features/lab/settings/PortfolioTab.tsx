import type { ReactNode } from "react";
import { NumField } from "../OperandEditor";
import { setExecution } from "../spec-model";
import type { InstrumentContext } from "../execution-model";
import { portfolioEnabled } from "../portfolio-model";
import type { Catalogue, PositionLimits, Spec } from "../types";
import { cashWhy, ONE_INSTRUMENT, Why } from "./common";

const SHEET_MAX_OPEN = 500; // the team's AmiBroker Max. Open Positions

/** AmiBroker Analysis settings › Portfolio, in its order, under the strategy mode (`portfolio`, the PortfolioPanel). */
export function PortfolioTab({ spec, catalogue, instrument, onChange, portfolio }: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void; portfolio?: ReactNode }) {
  const p = spec.portfolio, limits = spec.execution?.limits ?? {}, cap = catalogue.portfolio?.max_open ?? 50;
  // the volume limit and previous-bar equity are execution.limits: money mode on one instrument (the portfolio engine refuses them)
  const limitsWhy = p ? ONE_INSTRUMENT : cashWhy(spec, catalogue, instrument);
  const openWhy = !p ? (portfolioEnabled(catalogue) ? "Needs a portfolio (Strategy mode above): one instrument holds one position." : "This engine does not offer portfolios")
    : cap < SHEET_MAX_OPEN ? `The lab allows up to ${cap} (AmiBroker sheet: ${SHEET_MAX_OPEN}).` : null;
  const setLimit = (key: keyof PositionLimits, v: number | boolean | undefined) => {
    const next: PositionLimits = { ...limits, [key]: v };
    if (v === undefined) delete next[key];
    onChange(setExecution(spec, { limits: Object.keys(next).length ? next : undefined }));
  };
  return <>
    <p className="cap lab-stab-intro">Several instruments with shared cash, and the limits on each position.</p>
    {portfolio}
    <fieldset className="lab-fieldset" data-lab-portfolio-tab>
      <legend>Portfolio-level settings</legend>
      <div className="lab-row">
        <NumField label="Max. Open Positions" value={p ? p.max_open ?? 1 : undefined} placeholder={p ? undefined : "1"} min={1} max={cap} step={1} disabled={!p}
          onChange={(n) => p && n !== undefined && onChange({ ...spec, portfolio: { ...p, max_open: n } })} />
        <Why text={openWhy} />
      </div>
      <div className="lab-row">
        {/* AmiBroker's 0 means no limit, so 0 or empty removes the key */}
        <NumField label="Limit trade size as % of entry bar volume (zero means no-limit)" value={limitsWhy ? undefined : limits.max_volume_pct} min={0} max={100} placeholder="0" disabled={!!limitsWhy}
          onChange={(n) => setLimit("max_volume_pct", n || undefined)} />
        {/* AmiBroker's default, and the engine's only behaviour (ledger.rs: a bar with no volume does not cap the size) */}
        <label className="lab-check" title="Always on in the lab: a bar with zero or missing volume does not limit the trade size.">
          <input type="checkbox" aria-label="Disable trade size limit when bar volume is zero" checked disabled readOnly /> Disable trade size limit when bar volume is zero <span className="cap">(always on in the lab)</span></label>
        <label className="lab-check"><input type="checkbox" aria-label="Use previous bar equity for position sizing" checked={!limitsWhy && limits.prev_bar_equity === true} disabled={!!limitsWhy}
          onChange={(e) => setLimit("prev_bar_equity", e.target.checked ? true : undefined)} /> Use previous bar equity for position sizing</label>
        <Why text={limitsWhy} />
      </div>
      {!limitsWhy && !!limits.max_volume_pct && <p className="cap">The volume limit reads the last complete bar when the order fills: the fill bar for a close fill, otherwise the bar before it, so it never looks ahead.</p>}
    </fieldset>
  </>;
}
