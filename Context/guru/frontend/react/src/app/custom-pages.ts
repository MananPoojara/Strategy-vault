import { createElement, type ComponentType } from "react";
import { IndicatorsPage } from "../pages/IndicatorsPage";
import { YtStrategiesPage } from "../pages/YtStrategiesPage";
import MaintenancePage, { maintenanceStyles } from "../pages/MaintenancePage";
import { ResearchPortal } from "../pages/ResearchPortal";
export interface PageRegistration {
  path: string;
  label: string;
  component: ComponentType;
  showInNavigation?: boolean;
}
/** The second developer registers new JSX pages here. Use a separate branch. */
export const customPages: PageRegistration[] = [
  { path: "/research", label: "Research", component: ResearchPortal, showInNavigation: true },
  { path: "/indicators", label: "Indicators", component: IndicatorsPage, showInNavigation: true },
  { path: "/yt-strategies", label: "YT Strategies", component: YtStrategiesPage, showInNavigation: true },
  { path: "/maintenance", label: "Maintenance", component: () => createElement("div", null, createElement("style", null, maintenanceStyles), createElement(MaintenancePage)), showInNavigation: false },
];
