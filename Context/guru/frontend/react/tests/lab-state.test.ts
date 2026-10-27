import { expect, it } from "vitest";
import { editorState, initialLabState, labReducer, runKey } from "../src/features/lab/lab-state";
import { defaultSpec, setAt, starterSpec } from "../src/features/lab/spec-model";
import { catalogueFixture as cat, strategyFixture } from "./fixtures/lab";
import { readLabQuery } from "../src/features/lab/url-state";

const ind = (id: string) => cat.indicators.find((i) => i.id === id)!;
const start = () => initialLabState(readLabQuery("#/indicators?tk=NIFTY%2050"));

it("starts from the query; a missing region falls back to the research region", () => {
  expect(start()).toMatchObject({ market: "in", ticker: "NIFTY 50", tf: "D", range: "3Y", chips: [], row: null, tried: 0 });
  expect(initialLabState(readLabQuery("#/indicators"), "us").market).toBe("us");
  expect(initialLabState(readLabQuery("#/indicators?region=gl")).market).toBe("gl");
});

it("caps price overlays at 4 and panes at 3, dropping the oldest", () => {
  let s = start();
  for (let i = 0; i < 5; i++) s = labReducer(s, { type: "chip-add", indicator: ind("ema") });
  for (let i = 0; i < 4; i++) s = labReducer(s, { type: "chip-add", indicator: ind("rsi") });
  expect(s.chips.filter((c) => c.id === "ema")).toHaveLength(4);
  expect(s.chips.filter((c) => c.id === "rsi")).toHaveLength(3);
  expect(s.chips[0].params).toEqual({ length: 9 });
  const key = s.chips[0].key;
  s = labReducer(s, { type: "chip-set", key, params: { length: 30 } });
  expect(s.chips.find((c) => c.key === key)!.params).toEqual({ length: 30 });
  expect(labReducer(s, { type: "chip-remove", key }).chips.some((c) => c.key === key)).toBe(false);
});

it("market change resets instrument, chips and the open row but keeps the rules", () => {
  let s = labReducer(start(), { type: "chip-add", indicator: ind("ema") });
  s = labReducer(s, { type: "open", row: strategyFixture(), spec: strategyFixture().spec });
  s = labReducer(s, { type: "spec", spec: setAt(s.spec, "exit.any[0].rhs.value", 9) });
  const next = labReducer(s, { type: "market", market: "us" });
  expect(next).toMatchObject({ market: "us", ticker: "", chips: [], row: null });
  expect(next.spec).toEqual(s.spec);
});

it("submit readiness: unchanged row, same instrument, fresh successful run", () => {
  const row = strategyFixture();
  let s = labReducer(start(), { type: "open", row, spec: row.spec });
  expect(editorState(s, "in")).toMatchObject({ unchanged: true, ranFresh: false });
  s = labReducer(s, { type: "ran", key: runKey(s.spec, s.market, s.ticker), combos: 0 });
  expect(editorState(s, "in")).toMatchObject({ unchanged: true, ranFresh: true });
  expect(s.tried).toBe(1);
  const edited = labReducer(s, { type: "spec", spec: setAt(s.spec, "exit.any[0].rhs.value", 6) });
  expect(editorState(edited, "in")).toMatchObject({ unchanged: false, ranFresh: false });
  const reordered = labReducer(s, { type: "row", row: { ...row, version: 4, spec: JSON.parse(JSON.stringify(row.spec, ["v", "exit", "any", "lhs", "s", "op", "rhs", "value", "entry", "all", "p", "length"])) } });
  expect(editorState(reordered, "in").unchanged).toBe(true);
  expect(labReducer(s, { type: "tried", n: 3 }).tried).toBe(4);
  expect(initialLabState(readLabQuery("#/indicators")).spec).toEqual(starterSpec()); // no rules yet (the default exit applies), trades at the signal bar's close
  expect(starterSpec()).toEqual({ v: 1, entry: { all: [] }, exit: { any: [] }, execution: { fill: "close" } });
});
it("uses a portfolio save/run identity independently of the chart ticker",()=>{
 const spec={...defaultSpec(),portfolio:{universe:{basket:"nifty50"},mode:"signals" as const}};
 const state={...start(),spec};expect(editorState(state,"in").ticker).toBe("portfolio:nifty50");
 expect(runKey(spec,"in","NIFTY 50")).toBe(runKey(spec,"in","NIFTY BANK"));
});

it("chip-look sets the per-plot looks and keeps the chip's params and comparison", () => {
  const s = { ...start(), chips: [{ key: "c1", id: "ema", params: { length: 20 }, vs: "SPX" }] };
  const next = labReducer(s, { type: "chip-look", key: "c1", looks: { ema: { style: "dashed" } } });
  expect(next.chips[0]).toEqual({ key: "c1", id: "ema", params: { length: 20 }, vs: "SPX", looks: { ema: { style: "dashed" } } });
  expect(labReducer(next, { type: "chip-look", key: "c1", looks: undefined }).chips[0].params).toEqual({ length: 20 });
});
