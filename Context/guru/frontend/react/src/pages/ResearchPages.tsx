import type { ViewSnapshot } from "../state/store";
import { ResearchMarkup } from "../components/ResearchMarkup";
// These presenters preserve reviewed rules, admission notices, charts and exports.
// The React-owned page boundary remounts when the immutable result revision changes.
export function AuthorPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function StrategiesPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function StrategyPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function OptimizationPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function ReadingPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
