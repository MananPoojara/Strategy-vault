import type { StrategyIndicators } from "../services/strategy-indicators";
import "./strategy-indicators.css";

/** Native React markup used by the research presenter. Checkbox changes are
 * connected to the chart controller after the immutable page has committed. */
export function StrategyIndicatorControls({ result }: { result: StrategyIndicators }) {
  return (
    <div className="strategy-indicators" data-strategy-indicators data-indicator-status={result.status}>
      {result.status === "ready" && (
        <div className="indicator-controls" role="group" aria-label="Strategy indicators">
          <span className="indicator-controls-label">Indicators</span>
          {result.series.map((series) => {
            const available = series.points.some((point) => point.value !== null);
            return (
              <label key={series.key} className="indicator-toggle" data-indicator-kind={series.kind}>
                <input type="checkbox" defaultChecked={available} disabled={!available}
                  data-indicator-toggle={series.key} />
                <i aria-hidden="true" className={`indicator-swatch indicator-swatch-${series.key}`} />
                {series.label}
                <output data-indicator-value={series.key}>{available ? "" : "Warmup incomplete"}</output>
              </label>
            );
          })}
        </div>
      )}
      <p className="indicator-note">{result.note}</p>
      {result.series.filter((series) => series.placement === "pane").map((series) => (
        <div className="indicator-pane-wrap" key={series.key} data-indicator-pane-wrap={series.key}>
          <div className="indicator-pane-caption">
            <strong>{series.label}</strong>
            <span>{series.thresholds.map((line) => `${line.label} ${line.value}`).join(" · ")}</span>
          </div>
          <div className="indicator-pane" data-indicator-pane={series.key}
            aria-label={`${series.label} indicator chart`} />
        </div>
      ))}
    </div>
  );
}
