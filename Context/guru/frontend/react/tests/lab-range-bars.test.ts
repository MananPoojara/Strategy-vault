import { expect, it } from "vitest";
import { autoRange, rangeBars } from "../src/features/lab/timeless";

it("range bars: every finished bar spans exactly the range, closes at the extreme that completed it, the next opens there", () => {
  // one up day 100 → 112: path 100 (open), 99 (low), 112 (high), 111 (close), range 5
  const bars = rangeBars(["2026-01-05"], [100], [112], [99], [111], 5);
  const done = bars.slice(0, -1);
  expect(done.every((b) => Math.abs(b.high - b.low - 5) < 1e-9)).toBe(true);
  expect(done.map((b) => b.close)).toEqual([104, 109]); // low 99 + 5, then + 5
  expect(bars[1].open).toBe(104);
  expect(bars.at(-1)!.close).toBe(111); // the bar still forming
});

it("a down day walks open → high → low → close and makes down bars", () => {
  const bars = rangeBars(["2026-01-05"], [100], [101], [90], [91], 4);
  expect(bars.slice(0, -1).every((b) => !b.up)).toBe(true);
  expect(bars[0].close).toBe(97); // high 101 − 4
});

it("the default range is half the recent average daily range, rounded to 2 figures", () => {
  expect(autoRange([110, 112], [100, 100])).toBe(5.5); // (10 + 12) / 2 / 2
});
