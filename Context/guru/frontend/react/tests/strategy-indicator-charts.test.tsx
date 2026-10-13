import { afterEach, describe, expect, it, vi } from "vitest";
import { buildStrategyIndicators } from "../src/services/strategy-indicators";
import { indicatorControlsHTML, mountStrategyIndicators } from "../src/services/strategy-indicator-charts";

const result = () => buildStrategyIndicators({ id: "dmi_1", fullHistory: true,
  prices: { d: ["2020-01-01", "2020-01-02", "2020-01-03", "2020-01-04"],
    o: [10, 11, 10, 12], h: [11, 12, 11, 13], l: [9, 10, 9, 11], c: [10, 11, 10, 12] },
  parameters: { trend: 3, rsi_n: 2, exit_rsi: 65, di_n: 2, di_lt: 5 },
});

function mockChart() {
  type Range = { from: number; to: number } | null;
  let range: Range = { from: 1, to: 3 };
  const ranges = new Set<(range: Range) => void>();
  const crosshairs = new Set<(event: { time?: string }) => void>();
  const lines: ReturnType<typeof mockLine>[] = [];
  const timeScale = {
    getVisibleLogicalRange: () => range,
    setVisibleLogicalRange: vi.fn((next: Exclude<Range, null>) => { range = next; for (const listener of ranges) listener(range); }),
    subscribeVisibleLogicalRangeChange: (listener: (range: Range) => void) => ranges.add(listener),
    unsubscribeVisibleLogicalRangeChange: (listener: (range: Range) => void) => ranges.delete(listener),
  };
  const chart = {
    addLineSeries: vi.fn(() => { const line = mockLine(); lines.push(line); return line; }),
    removeSeries: vi.fn(),
    timeScale: () => timeScale,
    subscribeCrosshairMove: (listener: (event: { time?: string }) => void) => crosshairs.add(listener),
    unsubscribeCrosshairMove: (listener: (event: { time?: string }) => void) => crosshairs.delete(listener),
    setCrosshairPosition: vi.fn(), clearCrosshairPosition: vi.fn(), resize: vi.fn(), remove: vi.fn(),
  };
  return { chart, lines, ranges, crosshairs, timeScale, move: (time?: string) => { for (const listener of crosshairs) listener({ time }); } };
}
function mockLine() { return { setData: vi.fn(), applyOptions: vi.fn(), createPriceLine: vi.fn() }; }

afterEach(() => { document.body.innerHTML = ""; vi.restoreAllMocks(); });

describe("strategy indicator companion charts", () => {
  it("preserves a hidden indicator when the price chart is remounted for another timeframe", () => {
    const host = document.createElement('div');host.innerHTML=indicatorControlsHTML(result());document.body.append(host);
    const root=host.firstElementChild as HTMLElement;
    root.querySelector<HTMLInputElement>('[data-indicator-toggle="plus-di"]')!.checked=false;
    const price=mockChart(),panes=[mockChart(),mockChart()];let next=0;
    const controller=mountStrategyIndicators({root,chart:price.chart,result:result(),library:{createChart:()=>panes[next++].chart}});
    expect(root.querySelector<HTMLElement>('[data-indicator-pane-wrap="plus-di"]')!.hidden).toBe(true);
    expect(panes[1].lines[0].applyOptions).toHaveBeenCalledWith({visible:false});
    controller.dispose();
  });
  it("renders named, accessible toggles and exact parameter captions into the existing React presenter", () => {
    const html = indicatorControlsHTML(result());
    expect(html).toContain('aria-label="Strategy indicators"');
    expect(html).toContain("SMA(3)");
    expect(html).toContain("RSI(2)");
    expect(html).toContain("+DI(2)");
    expect(html).toContain("Exit / entry cap 65");
    expect(html).not.toContain("onclick");
    expect(html).not.toContain("<script");
  });

  it("retains aligned warmup dates, synchronizes ranges and cleans every companion without removing the price chart", () => {
    const host = document.createElement("div"); host.innerHTML = indicatorControlsHTML(result()); document.body.append(host);
    const root = host.firstElementChild as HTMLElement;
    const price = mockChart(), panes = [mockChart(), mockChart()]; let paneIndex = 0;
    const controller = mountStrategyIndicators({ root, chart: price.chart, result: result(),
      library: { createChart: () => panes[paneIndex++].chart } });
    expect(paneIndex).toBe(2);
    expect(price.lines).toHaveLength(1);
    expect(panes[0].lines[0].setData.mock.calls[0][0]).toEqual([
      { time: "2020-01-01" }, { time: "2020-01-02" },
      { time: "2020-01-03", value: 50 }, { time: "2020-01-04", value: 100 - 100 / 6 },
    ]);
    price.timeScale.setVisibleLogicalRange({ from: 0, to: 2 });
    expect(panes[0].timeScale.getVisibleLogicalRange()).toEqual({ from: 0, to: 2 });
    panes[1].timeScale.setVisibleLogicalRange({ from: 2, to: 3 });
    expect(price.timeScale.getVisibleLogicalRange()).toEqual({ from: 2, to: 3 });
    expect(panes[0].timeScale.getVisibleLogicalRange()).toEqual({ from: 2, to: 3 });
    controller.dispose(); controller.dispose();
    expect(price.chart.removeSeries).toHaveBeenCalledOnce();
    expect(price.chart.remove).not.toHaveBeenCalled();
    for (const pane of panes) {
      expect(pane.chart.remove).toHaveBeenCalledOnce();
      expect(pane.ranges.size).toBe(0); expect(pane.crosshairs.size).toBe(0);
    }
    expect(price.ranges.size).toBe(0); expect(price.crosshairs.size).toBe(0);
  });

  it("updates indicator readouts and toggles the separate pane without mixing RSI with the price scale", () => {
    const host = document.createElement("div"); host.innerHTML = indicatorControlsHTML(result()); document.body.append(host);
    const root = host.firstElementChild as HTMLElement;
    const price = mockChart(), panes = [mockChart(), mockChart()]; let paneIndex = 0;
    const controller = mountStrategyIndicators({ root, chart: price.chart, result: result(),
      library: { createChart: () => panes[paneIndex++].chart },
      priceSeries: {}, pricePoints: [{ time: "2020-01-03", value: 10 }] });
    price.move("2020-01-03");
    expect(root.querySelector('[data-indicator-value="rsi"]')?.textContent).toBe("50.00");
    expect(panes[0].chart.setCrosshairPosition).toHaveBeenCalledWith(50, "2020-01-03", panes[0].lines[0]);
    panes[1].move("2020-01-03");
    expect(price.chart.setCrosshairPosition).toHaveBeenCalledWith(10, "2020-01-03", expect.anything());
    const toggle = root.querySelector<HTMLInputElement>('[data-indicator-toggle="rsi"]')!;
    toggle.checked = false; toggle.dispatchEvent(new Event("change", { bubbles: true }));
    expect(root.querySelector<HTMLElement>('[data-indicator-pane-wrap="rsi"]')!.hidden).toBe(true);
    expect(panes[0].lines[0].applyOptions).toHaveBeenLastCalledWith({ visible: false });
    controller.setVisible("rsi", true);
    expect(toggle.checked).toBe(true);
    expect(root.querySelector<HTMLElement>('[data-indicator-pane-wrap="rsi"]')!.hidden).toBe(false);
    price.move();
    expect(root.querySelector('[data-indicator-value="rsi"]')?.textContent).toBe("");
    expect(panes[0].chart.clearCrosshairPosition).toHaveBeenCalled();
    controller.dispose();
  });
});
