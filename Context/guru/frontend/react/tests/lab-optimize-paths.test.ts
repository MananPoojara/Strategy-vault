import { expect, it } from "vitest";
import { optimizablePaths } from "../src/features/lab/optimize-paths";
import { defaultSpec, setAt } from "../src/features/lab/spec-model";
import { catalogue7Fixture as cat } from "./fixtures/lab";
import type { Spec } from "../src/features/lab/types";
it("walks nested expressions, short rules and both entry levels with engine paths", () => {
  const s: Spec = { ...defaultSpec(), direction: "both", entry: { all: [{ any: [{ lhs: { avg: { s: "close" }, bars: 10 }, op: ">", rhs: { value: 1 } }] }], stop: { at: { highest: { s: "high" }, bars: 5 } } }, short_side: { entry: { all: [{ lhs: { s: "rsi", p: { length: 2 } }, op: ">", rhs: { value: 90 } }], limit: { at: { ref: { s: "close" }, bars: 2 } } }, exit: { any: [], stop_loss: { pct: 3 } } }, execution: { sizing: { risk_pct: 1 } } };
  const by = (path: string) => optimizablePaths(s, cat).find((p) => p.path === path);
  expect(by("entry.all[0].any[0].lhs.bars")).toMatchObject({ label: "Long entry 1 › any 1 › left › bars", group: "Long entry", min: 1, max: 1000, integer: true });
  expect(by("short_side.entry.all[0].lhs.p.length")).toMatchObject({ current: 2, integer: true, group: "Short entry" });
  expect(by("entry.stop.at.bars")?.group).toBe("Long entry order");
  expect(by("short_side.entry.limit.at.bars")?.group).toBe("Short entry order");
  expect(by("short_side.exit.stop_loss.pct")).toMatchObject({ min: 0.5, max: 99, step: 0.5 });
  expect(by("execution.sizing.risk_pct")).toMatchObject({ group: "Position size", min: 0.25, max: 100 });
});
it("lists supported distances, activation and scale-ins without offering excluded knobs", () => {
  let s = setAt(defaultSpec(), "entry.limit", { pct: 1 });
  s = setAt(s, "exit.trailing", { atr: 2, atr_length: 14, activate: { points: 4 } });
  s = setAt(s, "exit.breakeven", { after: { pct: 3 }, plus: { pct: 1 } });
  s = setAt(s, "entry.scale_in", { legs: [0.5, 0.5] });
  const paths = optimizablePaths(s, cat);
  expect(paths.find((p) => p.path === "entry.limit.pct")).toMatchObject({ min: 0, max: 50 });
  expect(paths.find((p) => p.path === "exit.trailing.activate.points")).toMatchObject({ current: 4, group: "Exits" });
  expect(paths.find((p) => p.path === "entry.scale_in.min_bars")?.current).toBe(1);
  expect(paths.some((p) => /atr_length|breakeven.plus|legs\[/.test(p.path))).toBe(false);
  expect(new Set(paths.map((p) => p.path)).size).toBe(paths.length);
});
it("walks all score expressions and exposes portfolio rank limits",()=>{
 const spec={...defaultSpec(),portfolio:{universe:{basket:"nifty50"},mode:"rotation" as const,max_open:3,score:{count:{lhs:{s:"rsi",p:{length:2}},op:"<",rhs:{value:5}},bars:3}}};
 const paths=optimizablePaths(spec,cat);expect(paths.map(p=>p.path)).toEqual(expect.arrayContaining(["portfolio.score.count.lhs.p.length","portfolio.score.count.rhs.value","portfolio.score.bars","portfolio.max_open","portfolio.worst_rank_held"]));
 expect(paths.find(p=>p.path==="portfolio.worst_rank_held")!.current).toBe(3);
});
