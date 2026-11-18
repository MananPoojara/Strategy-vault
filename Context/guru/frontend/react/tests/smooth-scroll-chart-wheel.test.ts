import { expect, it } from "vitest";
import { isChartWheelTarget } from "../src/app/smooth-scroll";
it("lets charts own the wheel: Lenis skips nodes inside a Lightweight Charts root, not the rest of the page", () => {
  const chart = document.createElement("div"); chart.className = "tv-lightweight-charts";
  const page = document.createElement("section"); page.className = "box";
  expect(isChartWheelTarget(chart)).toBe(true);
  expect(isChartWheelTarget(page)).toBe(false);
});
