import {describe, expect, it} from "vitest";
import {draftErrors, draftPatch} from "../src/components/StrategySettings";
import {returnDisplay} from "../src/services/return-display";

const run = {trades: [{open: false, ret: 0.1}, {open: false, ret: -0.05}, {open: true, ret: 0.2}]};
const draft = {equity: "1000000", sizeMode: "compound", sizePct: "50", alloc: "1000000", brokerage: "0.03", slippage: "2", rollover: "0", deadStock: "1", penny: "10"};

describe("backtest settings", () => {
  it("full account = today's numbers", () => {
    expect(returnDisplay(run, "fixed", null, null, {weight: 1}).value).toBeCloseTo(0.05, 12);
    expect(returnDisplay(run, "compound", null, null, {weight: 1}).value).toBeCloseTo(1.1 * 0.95 - 1, 12);
  });
  it("% of equity compounds the used share; fixed amount scales the sum", () => {
    expect(returnDisplay(run, "compound", null, null, {weight: 0.5}).value).toBeCloseTo(1.05 * 0.975 - 1, 12);
    expect(returnDisplay(run, "fixed", null, null, {weight: 0.1}).value).toBeCloseTo(0.005, 12);
  });
  it("brokerage % becomes engine bps; bad values are named", () => {
    expect(draftPatch(draft as never)).toMatchObject({report_mode: "compound", cost: "3", slippage: "2", dead_stock: "1", penny: "10", size_pct: "50"});
    expect(draftErrors(draft as never)).toEqual({});
    expect(Object.keys(draftErrors({...draft, sizeMode: "fixed", alloc: "2000000", brokerage: "6", sizePct: "0"} as never))).toEqual(["alloc", "brokerage"]);
  });
});
