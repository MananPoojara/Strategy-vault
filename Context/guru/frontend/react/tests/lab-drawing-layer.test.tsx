import { afterEach, expect, it, vi } from "vitest";
import { DrawingLayer } from "../src/features/lab/DrawingLayer";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);

it("draws nothing (instead of crashing the page) while the chart it drew on is being replaced", async () => {
  // a chart already removed by a settings change: lightweight-charts throws from its API ("reading 'dp'")
  const removed = { priceScale: () => ({ width: () => { throw new TypeError("Cannot read properties of undefined (reading 'dp')"); } }),
    timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn() }) } as unknown as LabChartApi;
  const host = await mount(<DrawingLayer chart={removed} series={{} as LabSeriesApi} times={["2024-01-01"]} drawings={[{ kind: "h", price: 100 }]}
    tool={null} color="#2962ff" height={300} onChange={vi.fn()} onDone={vi.fn()} />);
  expect(host.querySelector("[data-lab-draw-layer]")).toBeNull();
});

it("leaving a chart that a rebuild already removed does not throw", async () => {
  const ts = { subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(() => { throw new TypeError("removed"); }),
    logicalToCoordinate: () => 10, coordinateToLogical: () => 0 };
  const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ts } as unknown as LabChartApi;
  await mount(<DrawingLayer chart={chart} series={{ priceToCoordinate: () => 20, coordinateToPrice: () => 100 } as unknown as LabSeriesApi} times={["2024-01-01"]}
    drawings={[]} tool={null} color="#2962ff" height={300} onChange={vi.fn()} onDone={vi.fn()} />);
  await expect(unmount()).resolves.toBeUndefined();
});
