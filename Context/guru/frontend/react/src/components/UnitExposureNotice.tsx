import type {UnitExposureView} from '../services/unit-exposure-report';
import {UNIT_VISUAL_BASIS} from '../services/unit-exposure-presenter';
export function UnitExposureNotice({view}:{view:UnitExposureView}) {
  return <aside className="portfolio-basis" data-unit-exposure-notice aria-label="Reviewed index unit-exposure research model">
    <strong>Reviewed index rule · normalized unit exposure</strong>
    <p>SPX is the book’s cash-index illustration; NIFTY 50 is a separately declared adaptation. Daily same-close fills, first-close DEMA seeds and a flat selected start are declared conventions. No funded capital or share quantities are assumed.</p>
    <p>Gross compounds the same fills independently at zero cost. Audit contributions use net pretrade allocation. Terminal-open costs include an estimated second liquidation side; this is not a filled exit. Exact-zero closed trades are non-wins.</p>
    <p>Exposure and drawdown duration count observed sessions; Days Held uses calendar differences. {UNIT_VISUAL_BASIS}</p>
    <details><summary>Source and accounting basis</summary><p>{view.basis}</p><p>Book performance and native AmiBroker parity have not been reproduced.</p></details>
  </aside>;
}
