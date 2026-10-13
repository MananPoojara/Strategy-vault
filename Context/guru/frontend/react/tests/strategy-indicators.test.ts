import { describe, expect, it } from "vitest";
import {
  buildStrategyIndicators,
  bollingerPercentB,
  directionalPlus,
  simpleMovingAverage,
  wilderRsi,
  type PriceHistory,
} from "../src/services/strategy-indicators";

const prices = (closes: number[]): PriceHistory => ({
  d: closes.map((_, i) => `2020-01-${String(i + 1).padStart(2, "0")}`),
  o: closes,
  h: closes.map((c) => c + 1),
  l: closes.map((c) => c - 1),
  c: closes,
});

describe("indicators use the existing Rust engine conventions", () => {
  it("matches the connors.rs Wilder RSI hand calculation and seed", () => {
    const rsi = wilderRsi([10, 11, 10, 12], 2);
    expect(rsi.slice(0, 2)).toEqual([null, null]);
    expect(rsi[2]).toBe(50);
    expect(rsi[3]).toBeCloseTo(100 - 100 / 6, 12);
    expect(wilderRsi([10, 10, 10], 2)).toEqual([null, null, 50]);
    expect(wilderRsi([10, 11, 12], 2)[2]).toBe(100);
  });

  it("matches +DI seed excluding bar zero, true range gaps and tied moves", () => {
    // First two transitions: +DM 1, 0; TR 3, 12 -> +DI = 100*.5/7.5.
    const p: PriceHistory = {
      d: ["2020-01-01", "2020-01-02", "2020-01-03", "2020-01-04"],
      o: [9, 10, 13, 13], c: [9, 10, 13, 13],
      h: [10, 11, 15, 16], l: [8, 8, 3, 2],
    };
    // Transition 2 has +move=4 and -move=5; its true range is 12.
    // Seed +DM=.5, true range=(3+12)/2=7.5 => 6.66667.
    const di = directionalPlus(p, 2);
    expect(di.slice(0, 2)).toEqual([null, null]);
    expect(di[2]).toBeCloseTo(100 * 0.5 / 7.5, 12);
    // Tied up/down movement is zero; smoothed +DM=.25, TR=(7.5+14)/2.
    expect(di[3]).toBeCloseTo(100 * 0.25 / 10.75, 12);
  });

  it("does not shift SMA or RSI warmup when the visible backtest starts later", () => {
    const full = prices([10, 11, 10, 12, 11, 13]);
    const result = buildStrategyIndicators({
      id: "dmi_1", prices: full, fullHistory: true,
      displayDates: full.d.slice(4),
      parameters: { trend: 3, rsi_n: 2, exit_rsi: 65, di_n: 3, di_lt: 5 },
    });
    expect(result.status).toBe("ready");
    expect(result.series.find((s) => s.key === "trend")!.points[0].value).toBe(11);
    expect(result.series.find((s) => s.key === "rsi")!.points[0].value)
      .toBeCloseTo(wilderRsi(full.c, 2)[4]!, 12);
    expect(result.series.find((s) => s.key === "rsi")!.thresholds)
      .toEqual([{ value: 65, label: "Exit / entry cap", kind: "exit" }]);
  });

  it("reads the actual custom periods and thresholds, with distinct course and ETF R3 exits", () => {
    const px = prices([10, 11, 10, 12, 11, 13]);
    const course = buildStrategyIndicators({ id: "r3", prices: px,
      fullHistory: true, parameters: { trend: 4, ma: 3, rsi_n: 3,
        rsi_lt: 8, first_lt: 55, exit_rsi: 80 }, });
    expect(course.series.map((s) => s.label)).toEqual(["SMA(4)", "SMA(3)", "RSI(3)"]);
    expect(course.series[2].thresholds.map((x) => x.value)).toEqual([8, 55, 80]);
    const etf = buildStrategyIndicators({ id: "hpetf_r3_long", prices: px,
      fullHistory: true, parameters: { trend: 4, first: 60, entry: 10, exit: 70 } });
    expect(etf.series.map((s) => s.label)).toEqual(["SMA(4)", "RSI(2)"]);
    expect(etf.series[1].thresholds.map((x) => x.value)).toEqual([10, 60, 70]);
  });

  it("keeps missing warmup, invalid OHLC and unknown methods explicit", () => {
    const px = prices([10, 11, 10, 12]);
    expect(buildStrategyIndicators({ id: "dmi_1", prices: px, fullHistory: false,
      parameters: { trend: 200 } }).series).toEqual([]);
    expect(buildStrategyIndicators({ id: "dmi_1", prices: { ...px, c: [10] },
      fullHistory: true, parameters: {} }).status).toBe("unavailable");
    expect(buildStrategyIndicators({ id: "unreviewed_custom_indicator", prices: px,
      fullHistory: true, parameters: {} }).status).toBe("unsupported");
    expect(buildStrategyIndicators({ id: "dmi_1", prices: px, fullHistory: true,
      parameters: { trend: 200, rsi_n: 2, exit_rsi: 65 }, }).status).toBe("unavailable");
  });

  it("samples daily indicators at real weekly/monthly candle dates", () => {
    const px = prices([10, 11, 10, 12, 11, 13]);
    const result = buildStrategyIndicators({ id: "r3", prices: px,
      fullHistory: true, displayDates: [px.d[2], px.d[5]],
      parameters: { trend: 3, ma: 2, rsi_n: 2, rsi_lt: 10, first_lt: 60, exit_rsi: 75 } });
    expect(result.series[0].points).toEqual([
      { time: px.d[2], value: (10 + 11 + 10) / 3 },
      { time: px.d[5], value: 12 },
    ]);
    const unavailable = buildStrategyIndicators({ id: "r3", prices: px,
      fullHistory: true, displayDates: ["2020-01-30"], parameters: {} });
    expect(unavailable.status).toBe("unavailable");
  });

  it("does not invent auxiliary VIX series and retains partial coverage notice", () => {
    const px = prices([10, 11, 10, 12]);
    const result = buildStrategyIndicators({ id: "vix2", prices: px, fullHistory: true,
      parameters: { trend: 3, rsi_n: 2, exit_rsi: 65, rsi_lt: 30, vix_rsi: 90 } });
    expect(result.series.map((s) => s.key)).toEqual(["trend", "rsi"]);
    expect(result.note).toContain("Auxiliary market series");
  });

  it("rejects invalid periods and retains a gap rather than bridging warmup", () => {
    expect(() => simpleMovingAverage([1, 2, 3], 0)).toThrow();
    expect(simpleMovingAverage([1, 2, 3, 4], 3)).toEqual([null, null, 2, 3]);
  });

  it("matches the population Bollinger %b hand calculation and keeps flat bands unavailable", () => {
    expect(bollingerPercentB([1, 2, 3, 4, 5], 5, 1)[4])
      .toBeCloseTo((5 - (3 - Math.sqrt(2))) / (2 * Math.sqrt(2)), 12);
    expect(bollingerPercentB([10, 10, 10], 2, 1)).toEqual([null, null, null]);
    const px = prices([10, 11, 10, 12, 11, 13]);
    const result = buildStrategyIndicators({ id: "pctb_1", prices: px, fullHistory: true,
      parameters: { trend: 3, rsi_n: 2, exit_rsi: 65, rsi_lt: 10, pctb_n: 3, pctb_lt: 0 } });
    expect(result.series.at(-1)?.label).toBe("%b(3, 1σ)");
    expect(result.series.at(-1)?.thresholds).toEqual([{ value: 0, label: "Entry below", kind: "entry" }]);
  });

  it("plots summed RSI with the engine's separate sum period and preserves missing warmup", () => {
    const px = prices([10, 11, 10, 12, 11, 13]);
    const result = buildStrategyIndicators({ id: "rsi_3", prices: px, fullHistory: true,
      parameters: { trend: 3, rsi_n: 2, exit_rsi: 65, sum_n: 3, days: 2, sum_lt: 45 } });
    const sum = result.series.at(-1)!;
    const rsi = wilderRsi(px.c, 3);
    expect(sum.label).toBe("RSI(3) · 2-day sum");
    expect(sum.points.slice(0, 4).every((point) => point.value === null)).toBe(true);
    expect(sum.points[4].value).toBe(rsi[4]! + rsi[3]!);
  });
});
