import {describe, expect, it} from "vitest";
import {barsInMarket, compareRows, extendedRisk, maeFigures, monthlyReturns, strategyMetricRows, worstDrawdowns} from "../src/services/desk-stats";

describe("desk stats", () => {
  const dates = ["2020-01-01", "2020-01-02", "2020-02-03", "2020-02-04", "2020-03-02", "2020-03-03", "2020-04-01"];
  const equity = [1, 1.2, 0.9, 1.0, 1.3, 1.1, 1.2];

  it("finds drawdowns peak to trough to recovery, deepest first", () => {
    const dd = worstDrawdowns(dates, equity);
    expect(dd).toHaveLength(2);
    expect(dd[0]).toMatchObject({started: "2020-01-02", trough: "2020-02-03", recovered: "2020-03-02", drawdown: 0.9 / 1.2 - 1});
    expect(dd[0].recoveryDays).toBe(28);
    expect(dd[1]).toMatchObject({started: "2020-03-02", trough: "2020-03-03", recovered: null});
  });

  it("measures month-end returns of the equity curve", () => {
    const m = monthlyReturns(dates, equity);
    expect(m.map(x => +x.toFixed(4))).toEqual([0.2, -0.1667, 0.1, 0.0909]);
  });

  it("sums observed adverse moves of closed trades only", () => {
    const t = [{accounting: {mae_ret_observed: -0.02}}, {accounting: {mae_ret_observed: -0.05}}, {open: true, accounting: {mae_ret_observed: -0.5}}];
    expect(maeFigures(t)).toEqual({avg: -0.035, worst: -0.05, total: -0.07});
  });

  it("counts bars in the market, entry and exit bars included", () => {
    expect(barsInMarket(dates, [{entry: "2020-01-02", exit: "2020-02-03"}, {open: true, entry: "2020-03-03"}])).toBe(4);
    expect(barsInMarket(dates, [{entry: "bad", exit: "2020-02-03"}])).toBeNull();
  });

  it("names rows as the desk does and keeps engine values", () => {
    const rows = strategyMetricRows({stats: {net_profit: 3, trades: 10, car_mdd: 0.2}, gross_stats: {net_profit: 3.5}, trades: [], dates, equity});
    expect(rows.map(r => r[0])).toContain("Monthly Thickness %");
    expect(rows.find(r => r[0] === "Total Return %")!.slice(1, 3)).toEqual([3.5, 3]);
    expect(compareRows({total_return: 2, mar: 0.3, trades: 5}, {total_return: 1, mar: 0.1}).map(r => r[0])[0]).toBe("Return %");
    expect(compareRows({total_return: 2}, null)[0][2]).toBeNull();
    expect(extendedRisk({stats: {profit_factor: 2, win_rate: 0.5, payoff: 2}, dates, equity}).find(c => c.label === "CPC Index")!.value).toBe(2);
  });
});
