import {describe, expect, it} from "vitest";
import {basisStats, tradesheetSheet, type SheetInput} from "../src/services/tradesheet-workbook";

const trade = (entry: string, exit: string, gross: number, fee = 0, slip = 0, open = false) =>
  ({entry, exit, open, avg_px: 100, exit_px: 100 * (1 + gross), reason: "rsi", ret_full_precision: gross - fee - slip,
    accounting: {gross_ret: gross, fee_ret: fee, slippage_ret: slip, net_ret: gross - fee - slip, days_held: 5, mae_ret_observed: -0.01, mfe_ret_observed: 0.02}});
const base = (costs: SheetInput["costs"], trades: SheetInput["trades"]): SheetInput => ({strategy: "Test", symbol: "Nifty 50", currency: "₹",
  rules: {setup: ["a"], entry: ["b"], scaleIn: [], exit: ["c"]}, period: {from: "2010-01-01", to: "2020-01-01"}, costs,
  filters: {deadStock: false, penny: 0}, account: {equity: 1e6, compound: false, weight: 1}, trades});
const heads = (s: ReturnType<typeof tradesheetSheet>, r: number) => (s.rows[r] || []).map(c => c?.v).filter(v => v !== undefined && v !== "");

describe("tradesheet workbook", () => {
  it("no costs: one Profit % column", () => {
    const s = tradesheetSheet(base({brokerageBps: 0, slippageBps: 0}, [trade("2010-02-01", "2010-02-06", 0.02), trade("2012-02-01", "2012-02-06", -0.01)]));
    expect(heads(s, 10).slice(0, 2)).toEqual(["Profit %", "Days"]);
    expect(heads(s, 30)).toContain("P&L %");
    expect(heads(s, 30)).not.toContain("Slippage %");
  });
  it("slippage and brokerage get their own columns; net = gross - costs", () => {
    const s = tradesheetSheet(base({brokerageBps: 3, slippageBps: 2}, [trade("2010-02-01", "2010-02-06", 0.02, 0.0006, 0.0004), trade("2012-02-01", "2012-02-06", -0.01, 0.0006, 0.0004), trade("2013-01-01", "2013-01-05", 0.05, 0.0003, 0.0002, true)]));
    expect(heads(s, 10).slice(0, 5)).toEqual(["Gross %", "Slippage %", "Brokerage %", "Net %", "Days"]);
    expect(heads(s, 30).slice(6, 12)).toEqual(["Gross P&L %", "Days Held", "Slippage %", "Brokerage %", "Net P&L %", "Net P&L (₹)"]);
    expect(s.rows[11][1]?.v).toBeCloseTo(0.01, 12);           // gross total, closed only
    expect(s.rows[11][4]?.v).toBeCloseTo(0.01 - 0.002, 12);   // net total
    expect(s.rows[11][3]?.v).toBeCloseTo(0.0012, 12);         // brokerage total
    expect(s.rows[32][10]?.f).toBe("G33-I33-J33");
  });
  it("statistics as the site defines them", () => {
    const st = basisStats([0.1, -0.05, 0.02], [10, 10, 10], 1);
    expect(st.winRatio).toBeCloseTo(2 / 3); expect(st.dd).toBeCloseTo(-0.05);
    expect(st.expectancy).toBeCloseTo((0.06 * 2 / 3 - 1 / 3 * 0.05) / 0.05);
    expect(st.thickness).toBeCloseTo((0.07 / 3) / 10 * 30);
  });
});
