import { expect, it } from "vitest";
import { optimizablePaths } from "../src/features/lab/optimize-paths";
import { specProblems } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogue7Fixture as cat } from "./fixtures/lab";

const S = { v: 1, entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { value: 1 } }, { any: [{ lhs: { s: "close" }, op: "<", rhs: { value: 9 } }, { off: true, lhs: { s: "close" }, op: ">", rhs: { value: 2 } }] }] },
  exit: { any: [], dual_atr: { initial: { period: 5, mult: 2, price: "hl2" }, favour: { period: 7, mult: 4, price: "open" } } } } as Spec;

it("every rule's on/off switch and the Dual ATR numbers are optimizable", () => {
  const paths = optimizablePaths(S, cat);
  const by = (p: string) => paths.find((x) => x.path === p);
  expect(by("entry.all[0].off")).toMatchObject({ current: 0, min: 0, max: 1, step: 1, integer: true });
  expect(by("entry.all[1].any[1].off")).toMatchObject({ current: 1 });
  expect(by("entry.all[1].any[0].off")).toBeDefined();
  expect(by("exit.dual_atr.initial.period")).toMatchObject({ current: 5, min: 1, max: 500, integer: true });
  expect(by("exit.dual_atr.favour.period")).toMatchObject({ current: 7 });
  expect(by("exit.dual_atr.favour.mult")).toMatchObject({ current: 4, integer: false });
});

it("a strategy whose only exit is Dual ATR can run", () => {
  expect(specProblems(S, cat).filter((p) => p.includes("exit.any"))).toEqual([]);
});
