import { expect, it } from "vitest";
import { newStrategySpec } from "../src/features/lab/spec-model";
import { withPortfolioMode } from "../src/features/lab/portfolio-model";
import { changedSettings } from "../src/features/lab/defaults";

it("a new strategy follows the team's AFLs on circuits (UC no entry, LC exit waits), a portfolio included; only turning them off is a change", () => {
  const s = newStrategySpec();
  expect(s.execution).toMatchObject({ defer_exit_on_circuit: true, skip_entry_on_circuit: true });
  expect(changedSettings(s)).not.toEqual(expect.arrayContaining(["no circuit exit hold", "entry on an upper-circuit close"]));
  const port = withPortfolioMode(s, "signals").execution ?? {};
  expect([port.defer_exit_on_circuit, port.skip_entry_on_circuit]).toEqual([true, true]); // the portfolio engine runs them per member
  const off = { ...s, execution: { ...s.execution, defer_exit_on_circuit: false, skip_entry_on_circuit: false } };
  expect(changedSettings(off)).toEqual(expect.arrayContaining(["no circuit exit hold", "entry on an upper-circuit close"]));
});
