import { expect, it, vi } from "vitest";
import { amiWheel } from "../src/features/lab/charts";

const chartWith = (range: { from: number; to: number }) => {
  const set = vi.fn();
  const chart = { timeScale: () => ({ getVisibleLogicalRange: () => range, setVisibleLogicalRange: set }) } as never;
  return { chart, set };
};
const wheel = (el: HTMLElement, init: WheelEventInit) => { const e = new WheelEvent("wheel", { cancelable: true, ...init }); el.dispatchEvent(e); return e; };

it("AmiBroker wheel: scrolling pans the bars, Ctrl+scroll zooms around the cursor, and the page does not scroll", () => {
  const el = document.createElement("div");
  el.getBoundingClientRect = () => ({ left: 0, width: 100 }) as DOMRect;
  const { chart, set } = chartWith({ from: 0, to: 100 });
  const off = amiWheel(el, chart);
  const pan = wheel(el, { deltaY: 100 });
  expect(pan.defaultPrevented).toBe(true);
  expect(set).toHaveBeenLastCalledWith({ from: 8, to: 108 }); // 8 % of 100 bars later
  wheel(el, { deltaY: -100 });
  expect(set).toHaveBeenLastCalledWith({ from: -8, to: 92 });
  wheel(el, { deltaY: -100, ctrlKey: true, clientX: 50 }); // zoom in around the middle
  const z = set.mock.calls.at(-1)![0];
  expect(z.to - z.from).toBeCloseTo(100 / 1.15, 6);
  expect((z.from + z.to) / 2).toBeCloseTo(50, 6);
  off();
  set.mockClear();
  wheel(el, { deltaY: 100 });
  expect(set).not.toHaveBeenCalled(); // removed on dispose
});
