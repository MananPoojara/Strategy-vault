import type { ComponentType } from "react";
import { ResearchWork, ResearchYtsTab } from "../pages/ResearchWork";
export interface PageRegistration {
  path: string;
  label: string;
  component: ComponentType;
  showInNavigation?: boolean;
}
/** The second developer registers new JSX pages here. Use a separate branch. */
export const customPages: PageRegistration[] = [
  { path: "/research-yts", label: "Research YTS", component: ResearchYtsTab, showInNavigation: false }, // a tab of Research (20-11-2026)
  { path: "/research", label: "Research", component: ResearchWork, showInNavigation: true },
];
