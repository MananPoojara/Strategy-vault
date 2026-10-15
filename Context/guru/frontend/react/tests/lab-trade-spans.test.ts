import { expect, it } from "vitest";
import { tradeSpans } from "../src/features/lab/bars";
import { tint } from "../src/features/lab/charts";

it("shades each trade's bars by outcome, runs open trades to the end and skips unknown dates", () => {
  const dates = ["d0", "d1", "d2", "d3", "d4", "d5"], idx = new Map(dates.map((d, i) => [d, i])), barOf = [0, 1, 2, 3, 4, 5];
  const color = (t: { open: boolean; ret: number | null }) => (t.open ? "open" : (t.ret ?? 0) >= 0 ? "won" : "lost");
  const shade = tradeSpans([
    { entry: "d0", exit: "d1", open: false, ret: 0.02 }, { entry: "d2", exit: "d2", open: false, ret: -0.01 },
    { entry: "zz", exit: "d3", open: false, ret: 1 }, { entry: "d4", exit: "", open: true, ret: null },
  ], idx, barOf, 6, color);
  expect(shade).toEqual(["won", "won", "lost", undefined, "open", "open"]);
});

it("tints hex and rgb theme colours and never returns an opaque unknown format", () => {
  expect(tint("#112233", 0.5)).toBe("#11223380");
  expect(tint("rgb(1, 2, 3)", 0.14)).toBe("rgba(1, 2, 3, 0.14)");
  expect(tint("oklch(0.6 0.1 150)", 0.14)).toBe("transparent");
});
