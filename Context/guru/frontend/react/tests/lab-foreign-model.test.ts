import { expect, it } from "vitest";
import { defaultSpec, setAt } from "../src/features/lab/spec-model";
import { catalogue7Fixture } from "./fixtures/lab";
import { foreignOperandProblems, foreignProblems, foreignReferences, foreignOptions, withForeignField, type ForeignScope } from "../src/features/lab/foreign-model";
import type { Operand } from "../src/features/lab/types";
const scope: ForeignScope = { catalogue: { ...catalogue7Fixture, max_foreign: 4 }, market: "us", ticker: "SPY", options: [
 { ticker: "SPX", label: "S&P 500", disabled: false, hasVolume: false },
 { ticker: "SPY", label: "SPY", disabled: false, hasVolume: true },
 { ticker: "IG:DAX", label: "DAX", market: "gl", disabled: false, hasVolume: false },
], volatility: "CBOE VIX" };
it("changes one explicit foreign field immutably while retaining unrelated metadata", () => {
 const o = { s: "rsi", p: { length: 2 }, tf: "W", symbol: "SPX", fixup: "previous", future: { key: 1 } } as Operand;
 const before = structuredClone(o);
 const changed = withForeignField(o, "symbol", undefined);
 expect(changed).toEqual({ s: "rsi", p: { length: 2 }, tf: "W", fixup: "previous", future: { key: 1 } });
 expect(o).toEqual(before);
 expect(foreignOperandProblems(changed, scope).join(" ")).toContain("fixup requires");
});
it("discovers nested sides, state conditions, At levels and full expression portfolio scores", () => {
 let s = setAt(defaultSpec(), "entry.all[0].lhs", { abs: { s: "close", symbol: "SPX" } });
 s = setAt(s, "short_side.entry.all", [{ lhs: { count: { lhs: { s: "close", symbol: "CBOE VIX" }, op: ">", rhs: { value: 1 } }, bars: 3 }, op: ">", rhs: { value: 0 } }]);
 s = setAt(s, "entry.stop.at", { s: "ratio", vs: "SPY" });
 s = setAt(s, "portfolio.score", { add: [{ s: "ratio", vs: "SPX" }, { value: 1 }] });
 const refs = foreignReferences(s);
 expect(refs.map((r) => [r.path, r.field, r.ticker])).toEqual(expect.arrayContaining([
 ["entry.all[0].lhs.abs", "symbol", "SPX"], ["short_side.entry.all[0].lhs.count.lhs", "symbol", "CBOE VIX"],
 ["entry.stop.at", "vs", "SPY"], ["portfolio.score.add[0]", "vs", "SPX"],
 ]));
 expect(foreignProblems(s, scope)).toEqual([]);
});
it("validates pair requirements, D only, bars-held and score restrictions without rewriting", () => {
 expect(foreignOperandProblems({ s: "correlation" }, scope).join(" ")).toContain("Compared with");
 expect(foreignOperandProblems({ s: "ratio", vs: "SPX", tf: "W" }, scope).join(" ")).toContain("daily");
 expect(foreignOperandProblems({ s: "close", vs: "SPX" }, scope).join(" ")).toContain("pair indicators");
 expect(foreignOperandProblems({ s: "bars_held", symbol: "SPX" }, scope).join(" ")).toContain("Bars held");
 const s = setAt(defaultSpec(), "portfolio.score", { ref: { s: "close", symbol: "SPX" }, bars: 1 });
 expect(foreignProblems(s, scope).join(" ")).toContain("portfolio.score");
 expect(foreignProblems(s, scope).join(" ")).toContain("cannot read another symbol");
});
it("bounds unique symbol/vs names including the current ticker and rejects malformed names", () => {
 let s = defaultSpec();
 s = setAt(s, "entry.all", ["A", "B", "C", "D", "E"].map((symbol) => ({ lhs: { s: "close", symbol }, op: ">", rhs: { value: 0 } })));
 expect(foreignProblems(s, scope).join(" ")).toContain("reads 5 instruments; the limit is 4");
 for (const symbol of ["", "é".repeat(41), "A\u0000B", 7]) expect(foreignOperandProblems({ s: "close", symbol }, scope).join(" ")).toContain("instrument name");
 expect(foreignOperandProblems({ s: "close", symbol: "IG:DAX" }, scope).join(" ")).toContain("same market");
 expect(foreignOptions(scope).map((o) => o.ticker)).toEqual(["CBOE VIX", "SPX", "SPY"]);
});
it("keeps unsupported saved fields and rejects unknown symbols with a useful reason", () => {
 expect(foreignOperandProblems({ s: "close", symbol: "UNKNOWN" }, scope).join(" ")).toContain("not listed");
 expect(foreignOperandProblems({ s: "close", symbol: "SPX", fixup: "future" }, scope).join(" ")).toContain("none or previous");
 const old = { ...scope, catalogue: catalogue7Fixture };
 expect(foreignOperandProblems({ s: "close", symbol: "SPX" }, old).join(" ")).toContain("not advertised");
 expect(foreignProblems(setAt(defaultSpec(), "portfolio.score", { value: 1 }), scope).join(" ")).toContain("series");
 expect(foreignProblems(setAt(defaultSpec(), "portfolio.score", { abs: { s: "bars_held" } }), scope).join(" ")).toContain("Bars held");
});
it("describes each server-authoritative use instead of collapsing double alignment",async()=>{
 const {foreignDescription}=await import("../src/features/lab/foreign-model");
 const row={ticker:"IG:DAX",type:"cfd",alignment:"prior_date" as const,fixup:true,first:"2020-01-01",last:"2024-01-01",uses:[{path:"entry.all[0].lhs",source:"HSI:HSI",alignment:"prior_date" as const,fixup:true},{path:"entry.all[1].lhs.vs",source:"IG:FTSE",alignment:"prior_date" as const,fixup:false}]};
 const text=foreignDescription(row);expect(text).toContain("HSI:HSI");expect(text).toContain("IG:FTSE");expect(text).toContain("gaps kept");expect(text).toContain("previous available date");
});
it("labels implicit portfolio sources as each member instead of the legacy reference field",async()=>{
 const {foreignDescription}=await import("../src/features/lab/foreign-model");
 const row={ticker:"SPX",type:"index",alignment:"same_date" as const,fixup:false,first:"2024-01-01",last:"2024-02-01",uses:[{path:"entry.all[0].lhs",source:"SPX",source_scope:"portfolio_member" as const,alignment:"same_date" as const,fixup:false}]};
 expect(foreignDescription(row)).toContain("each portfolio member → SPX");expect(foreignDescription({...row,uses:row.uses.map(({source_scope,...u})=>u)})).toContain("SPX → SPX");
});
