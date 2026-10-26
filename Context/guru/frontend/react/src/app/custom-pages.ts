import type { ComponentType } from "react";
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
];
