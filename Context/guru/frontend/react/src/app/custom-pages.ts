import type { ComponentType } from "react";
import { ResearchYts } from "../pages/ResearchYts";
export interface PageRegistration {
  path: string;
  label: string;
  component: ComponentType;
  showInNavigation?: boolean;
}
/** The second developer registers new JSX pages here. Use a separate branch. */
export const customPages: PageRegistration[] = [
  { path: "/research-yts", label: "Research YTS", component: ResearchYts, showInNavigation: true },
];
