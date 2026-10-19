import { expect, it } from "vitest";
import { int, num, pct } from "../src/features/lab/format";

it("prints values that round to zero without a minus sign; dashes for missing values", () => {
  expect([pct(-0.0001), pct(-0.00001, 2), pct(-0.012), pct(0)]).toEqual(["0.0%", "0.00%", "-1.2%", "0.0%"]);
  expect([num(-0.001), num(-1234.5), int(-0.4), int(-2)]).toEqual(["0.00", "-1,234.50", "0", "-2"]);
  expect([pct(null), num(NaN), int(undefined)]).toEqual(["—", "—", "—"]);
});
