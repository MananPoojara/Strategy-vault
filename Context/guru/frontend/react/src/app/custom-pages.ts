import type { ComponentType } from "react";
import { IndicatorsPage } from "../pages/IndicatorsPage";
import { YtStrategiesPage } from "../pages/YtStrategiesPage";
export interface PageRegistration {
  path: string;
  label: string;
  component: ComponentType;
  showInNavigation?: boolean;
}
/** The second developer registers new JSX pages here. Use a separate branch. */
export const customPages: PageRegistration[] = [];
customPages.push(
  { path: "/indicators", label: "Indicators", component: IndicatorsPage, showInNavigation: true },
  { path: "/yt-strategies", label: "YT Strategies", component: YtStrategiesPage, showInNavigation: true },
);
