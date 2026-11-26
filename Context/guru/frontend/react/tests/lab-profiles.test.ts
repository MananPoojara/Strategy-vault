import { expect, it } from "vitest";
import { periods, tpoProfile, valueArea, volumeProfile, type ProfileBars } from "../src/features/lab/profiles";

const bars = (rows: [string, number, number, number, number, number | null][]): ProfileBars =>
  ({ d: rows.map((r) => r[0]), o: rows.map((r) => r[1]), h: rows.map((r) => r[2]), l: rows.map((r) => r[3]), c: rows.map((r) => r[4]), v: rows.map((r) => r[5]) });

it("spreads each bar's volume over its high–low range, up bars on the up side, and keeps the total", () => {
  // two bars over 100–110: an up bar 100–110 (1000) and a down bar 105–110 (500)
  const b = bars([["2026-01-05", 100, 110, 100, 108, 1000], ["2026-01-06", 109, 110, 105, 106, 500]]);
  const p = volumeProfile(b, 0, 1, 10)!;
  expect(p.lo).toBe(100); expect(p.step).toBe(1);
  expect(p.up.reduce((a, x) => a + x, 0)).toBeCloseTo(1000); expect(p.down.reduce((a, x) => a + x, 0)).toBeCloseTo(500);
  expect(p.up[0]).toBeCloseTo(100); expect(p.down[0]).toBe(0); expect(p.down[9]).toBeCloseTo(100); // 500 over 5 rows
  expect(p.poc).toBeGreaterThanOrEqual(5); // the rows both bars share hold the most volume
});

it("has no volume profile without volume (an index without volume)", () => {
  expect(volumeProfile(bars([["2026-01-05", 1, 2, 1, 2, null]]), 0, 0)).toBeNull();
});

it("Value Area: from the POC, the larger neighbouring row joins first, until 70%", () => {
  const rows = [1, 2, 10, 30, 20, 5, 1, 1];
  expect(valueArea(rows, 3, 0.7)).toEqual([3, 4]); // total 70, 70% = 49: POC 30 + the larger neighbour 20 = 50
  expect(valueArea(rows, 3, 0.8)).toEqual([2, 4]); // 80% = 56: the 10 below joins next
  expect(valueArea([5], 0)).toEqual([0, 0]);
});

it("TPO: each bar is a letter in every row its range touches; the POC is the busiest row", () => {
  const b = bars([["2026-01-05", 0, 10, 0, 5, null], ["2026-01-06", 0, 6, 4, 5, null], ["2026-01-07", 0, 6, 4, 5, null]]);
  const p = tpoProfile(b, 0, 2, 10)!;
  expect(p.letters![0]).toEqual(["A"]);
  expect(p.letters![4]).toEqual(["A", "B", "C"]);
  expect(p.poc).toBe(4); // rows 4 and 5 tie; TradingView takes the one nearer the middle
  expect(p.ib).toEqual([0, 10]);
});

it("one row size for every profile: a fixed step aligns rows to a common grid", () => {
  const b = bars([["2026-01-05", 101, 104, 101, 103, 100]]);
  const p = volumeProfile(b, 0, 0, 24, 0.7, 2)!;
  expect(p.lo).toBe(100); expect(p.step).toBe(2); expect(p.up).toHaveLength(2);
});

it("groups bars by calendar week (Monday start) and month", () => {
  const d = ["2026-01-02", "2026-01-05", "2026-01-09", "2026-01-12", "2026-02-02"];
  expect(periods(d, "W")).toEqual([[0, 0], [1, 2], [3, 3], [4, 4]]);
  expect(periods(d, "M")).toEqual([[0, 3], [4, 4]]);
});
