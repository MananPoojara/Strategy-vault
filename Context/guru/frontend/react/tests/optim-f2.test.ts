import { describe, expect, it } from "vitest";
import { AMI_COLS, amiStats, resultSheet, rulesSheet, sheetNames, type RunInput } from "../src/services/optim-f2";

// Six bars; capital 1000. Trade A buys 10 at 100 on bar 1, exits at 105 on bar 3 (+50); trade B buys 10 at 100 on bar 4,
// exits at 95 on bar 5 (-50). Money equity at the closes: 1000, 1000, 1100, 1050, 1050, 1000.
const RUN: RunInput = {
  dates: ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-06"],
  close: [100, 100, 110, 105, 100, 100],
  equity: [1, 1, 1.1, 1.05, 1.05, 1],
  capital: 1000,
  costRate: 0,
  trades: [
    { dir: 1, legs: [[1, 100, 1]], exit_i: 3, exit_px: 105, bars: 2, open: false, units: 10 },
    { dir: 1, legs: [[4, 100, 1]], exit_i: 5, exit_px: 95, bars: 1, open: false, units: 10 },
  ],
};

describe("Optimisation F2 statistics", () => {
  const s = amiStats(RUN);

  it("reports money, drawdowns and exposure like AmiBroker", () => {
    expect(s["Net Profit"]).toBeCloseTo(0, 9);
    expect(s["Max. Sys Drawdown"]).toBeCloseTo(-100, 9);
    expect(s["Max. Sys % Drawdown"]).toBeCloseTo(-100 / 11, 9);
    // the larger fall from a trade's own peak: trade A 100 -> 50, as % of its 1000 entry value
    expect(s["Max. Trade Drawdown"]).toBeCloseTo(-50, 9);
    expect(s["Max. Trade % Drawdown"]).toBeCloseTo(-5, 9);
    // position value / equity on bars 1, 2 (trade A) and 4 (trade B), over six bars
    expect(s["Exposure %"]).toBeCloseTo((1 + 1 + 1000 / 1050) / 6 * 100, 9);
    expect(s["Ulcer Index"]).toBeCloseTo(Math.sqrt((2 * (50 / 11) ** 2 + (100 / 11) ** 2) / 6), 9);
  });

  it("counts winners and losers, with AmiBroker's inclusive bars held", () => {
    expect([s["# Trades"], s["# of winners"], s["# of losers"]]).toEqual([2, 1, 1]);
    expect([s["Profit Factor"], s["Payoff Ratio"], s["% of Winners"]]).toEqual([1, 1, 50]);
    expect([s["W. Avg % Profit"], s["L. Avg % Loss"], s["Avg Bars Held"]]).toEqual([5, -5, 2.5]);
    expect(s["Sharpe Ratio"]).toBeNull(); // AmiBroker's definition is not reproduced
  });

  it("charges the side cost on entry and exit", () => {
    const c = amiStats({ ...RUN, costRate: 0.001 });
    expect(c["W. Tot. Profit"]).toBeCloseTo(50 - 10 * (100 + 105) * 0.001, 9);
  });
});

describe("Optimisation F2 workbook", () => {
  it("names sheets within Excel's rules and keeps them unique", () => {
    expect(sheetNames(["Fri L>Mon L.UpClose", "Fri L>Mon L.UpClose", "a/b:c", "x".repeat(40)])).toEqual(
      ["Fri L>Mon L.UpClose", "Fri L>Mon L.UpClose~2", "a-b-c", "x".repeat(31)]);
  });

  it("puts parameters after the AmiBroker columns and the team's formulas after them", () => {
    const sh = resultSheet({ name: "S", paramNames: ["N candles"], rows: [{ no: 7, stats: amiStats(RUN), params: [3] }] });
    expect(AMI_COLS.length).toBe(37);
    expect([sh.cells.A1.v, sh.cells.AK1.v, sh.cells.AL1.v, sh.cells.AM1.v]).toEqual(["No.", "L. Avg. Bars Held", "N candles", "Monthly Thickness"]);
    expect([sh.cells.A2.v, sh.cells.AL2.v]).toEqual([7, 3]);
    expect(sh.cells.AM2.f).toBe("X2/(Y2-1)*21");
    expect(sh.cells.AR2.f).toBe("(AO2/(Y2-1))*21"); // Final M. Thk uses the Final Avg column
    expect(sh.cells.AM2.v).toBe(0);
    expect(sh.cells.AV2).toMatchObject({ f: "V2*Y2", v: 5 });
  });

  it("lists the run and every setting on the Rules sheet", () => {
    const r = rulesSheet({ index: "Nifty 50", from: "2008-01-31", to: "2026-10-16", capital: 1000000, brokerage: "0 bps", strategy: "Monday Reversal", aflPath: "x.afl",
      settings: [["Entry condition", "Fri O>Mon H"], ["N candles", "(1,10,1)"]], note: "n" });
    expect([r.cells.B1.v, r.cells.B3.v, r.cells.C3.v]).toEqual(["Nifty 50", 39478, 46311]);
    expect([r.cells.B14.v, r.cells.C15.v]).toEqual(["Entry condition", "(1,10,1)"]);
  });
});
