import type { ViewSnapshot } from "../state/store";
import { ResearchMarkup } from "../components/ResearchMarkup";
import { StrategySettings } from "../components/StrategySettings";
// These presenters preserve reviewed rules, admission notices, charts and exports.
// The React-owned page boundary remounts when the immutable result revision changes.
export function AuthorPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function StrategiesPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function StrategyPage({ snapshot }: { snapshot: ViewSnapshot }) {
  const mode = snapshot.state.report_mode === "compound" ? "compound" : "fixed";
  const basket = snapshot.state.run_scope === "basket";
  return <ResearchMarkup html={snapshot.html} slots={{
    "strategy-settings": <StrategySettings
      mode={mode}
      disabledReason={basket
        ? "Basket returns use the equal-slot portfolio calculation. Closed-trade compounding applies only to one instrument."
        : undefined}
      onApply={report_mode => window.VaultRuntime.patch({report_mode})}
    />,
  }} />;
}
export function OptimizationPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function ReadingPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
