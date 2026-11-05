import { expect, it } from "vitest";
import { newStrategySpec } from "../src/features/lab/spec-model";
import { withPortfolioMode } from "../src/features/lab/portfolio-model";
import { changedSettings } from "../src/features/lab/defaults";

it("a new strategy starts with circuits off (most team AFLs leave them out); turning them on is a change, a portfolio keeps them per member", () => {
  const s = newStrategySpec();
  expect([s.execution?.defer_exit_on_circuit, s.execution?.skip_entry_on_circuit]).toEqual([undefined, undefined]);
  expect(changedSettings(s)).not.toEqual(expect.arrayContaining(["circuit exit hold", "no entry on a circuit close"]));
  const on = { ...s, execution: { ...s.execution, defer_exit_on_circuit: true, skip_entry_on_circuit: true } };
  expect(changedSettings(on)).toEqual(expect.arrayContaining(["circuit exit hold", "no entry on a circuit close"]));
  const port = withPortfolioMode(on, "signals").execution ?? {};
  expect([port.defer_exit_on_circuit, port.skip_entry_on_circuit]).toEqual([true, true]); // the portfolio engine runs them per member
});
