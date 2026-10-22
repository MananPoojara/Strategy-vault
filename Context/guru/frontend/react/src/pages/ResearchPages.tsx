import {PristineStrategyReport} from '../components/PristineStrategyReport';
import type { ViewSnapshot } from "../state/store";
import { ResearchMarkup } from "../components/ResearchMarkup";
import { StrategySettings } from "../components/StrategySettings";
import {SourceAccountNotice} from "../components/SourceAccountNotice";
import {UnitExposureNotice} from '../components/UnitExposureNotice';
import {RulesVisualGuide} from "../components/RulesVisualGuide";
import {useLayoutEffect, useMemo, useState} from "react";
import {prepareStrategyReport, REPORT_TABS, type ReportTabId} from "../services/strategy-report-layout";
// These presenters preserve reviewed rules, admission notices, charts and exports.
// The React-owned page boundary remounts when the immutable result revision changes.
export function AuthorPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function StrategiesPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
export function StrategyPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return snapshot.strategyPristine ? <PristineStrategyReport key={snapshot.revision} snapshot={snapshot.strategyPristine} chrome={snapshot.html}/> : <LegacyStrategyPage snapshot={snapshot}/>;
}
function LegacyStrategyPage({ snapshot }: { snapshot: ViewSnapshot }) {
  const mode = snapshot.state.report_mode === "compound" ? "compound" : "fixed";
  const basket = snapshot.state.run_scope === "basket";
  const report = useMemo(() => prepareStrategyReport(snapshot.html), [snapshot.html]);
  const [selectedTab, setSelectedTab] = useState<ReportTabId>("equity");
  const activeTab = report.tabs.includes(selectedTab) ? selectedTab : "equity";
  const tabs = REPORT_TABS.filter(tab => report.tabs.includes(tab.id));
  useLayoutEffect(() => {
    if (tabs.length) window.VaultRuntime?.activateReportTab?.();
  }, [activeTab, snapshot.revision, tabs.length]);
  return <ResearchMarkup html={snapshot.html} report={report} activeReportTab={activeTab} slots={{
    "report-tabs": <div className="report-tabs" role="tablist" aria-label="Backtest report views">
      {tabs.map((tab, index) => <button key={tab.id} type="button" role="tab"
        id={`report-tab-${tab.id}`} aria-controls={`report-panel-${tab.id}`}
        aria-selected={activeTab === tab.id} tabIndex={activeTab === tab.id ? 0 : -1}
        onClick={() => {if (!document.body.classList.contains("pdf-capture")) setSelectedTab(tab.id);}} onKeyDown={event => {
          if (document.body.classList.contains("pdf-capture")) return;
          const next = event.key === "ArrowRight" ? (index + 1) % tabs.length
            : event.key === "ArrowLeft" ? (index + tabs.length - 1) % tabs.length
            : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : null;
          if (next === null) return;
          event.preventDefault(); setSelectedTab(tabs[next].id);
          const button = document.getElementById(`report-tab-${tabs[next].id}`);
          button?.focus(); button?.scrollIntoView?.({block: "nearest", inline: "nearest"});
        }}>{tab.label}</button>)}
    </div>,
    "visual-rules": snapshot.strategyRules ? <RulesVisualGuide model={snapshot.strategyRules} /> : null,
    "strategy-settings": <StrategySettings
      mode={mode}
      sourceAccount={snapshot.strategyAccount || false}
      unitExposure={Boolean(snapshot.strategyUnit)}
      disabledReason={basket
        ? "Basket returns use the equal-slot portfolio calculation. Closed-trade compounding applies only to one instrument."
        : undefined}
      onApply={report_mode => window.VaultRuntime.patch({report_mode})}
    />,
    "source-account-notice": snapshot.strategyAccount ? <SourceAccountNotice account={snapshot.strategyAccount} /> : null,
    "unit-exposure-notice": snapshot.strategyUnit ? <UnitExposureNotice view={snapshot.strategyUnit} /> : null,
  }} />;
}
export function OptimizationPage({ snapshot }: { snapshot: ViewSnapshot }) {
  if (snapshot.strategyPristine) return <PristineStrategyReport key={snapshot.revision} snapshot={snapshot.strategyPristine} chrome={snapshot.html}/>;
  return <ResearchMarkup html={snapshot.html} slots={{
    "source-account-notice": snapshot.strategyAccount ? <SourceAccountNotice account={snapshot.strategyAccount} /> : null,
    "unit-exposure-notice": snapshot.strategyUnit ? <UnitExposureNotice view={snapshot.strategyUnit} /> : null,
  }} />;
}
export function ReadingPage({ snapshot }: { snapshot: ViewSnapshot }) {
  return <ResearchMarkup html={snapshot.html} />;
}
