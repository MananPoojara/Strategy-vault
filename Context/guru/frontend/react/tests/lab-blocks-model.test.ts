import { describe, expect, it } from "vitest";
import {
  FNS, FormulaParseError, ago, emptyBlocks, emptyKeys, fn, generate, getAt, indBlock, keyAt, num, paletteItems, param, parseFormula, place, price,
  problems, readBlocks, ref, renameLet, searchPalette, setAt, starterBlocks, type BlockNode, type Blocks, type FnId,
} from "../src/features/lab/blocks-model";
import { FORMULA_EXAMPLES } from "../src/features/lab/formula";
import { catalogueFixture as cat } from "./fixtures/lab";

const inds = cat.indicators;
const plot = (expr: BlockNode | null, name = "X", onPrice = false): Blocks => ({ v: 1, lets: [], plots: [{ name, onPrice, expr }] });
const text = (e: BlockNode | null) => generate(plot(e)).text.replace(/^Plot\(|, "X"\);$/g, "");
const C = price("C"), O = price("O");
const roundTrip = (src: string) => generate(parseFormula(src, inds)).text;

describe("generation", () => {
  it("emits every leaf kind", () => {
    expect(text(num(14))).toBe("14");
    expect(text(num(2.5))).toBe("2.5");
    expect(text({ k: "bool", v: true })).toBe("true");
    expect(text({ k: "bool", v: false })).toBe("false");
    for (const f of ["O", "H", "L", "C", "V", "Avg", "hl2", "hlc3", "ohlc4"] as const) expect(text(price(f))).toBe(f);
    expect(text(param("Fast", 12, 2, 50, 1))).toBe('Param("Fast", 12, 2, 50, 1)');
    expect(text(param('Say "hi" \\ there', 1, 0, 2, 0.5))).toBe('Param("Say \\"hi\\" \\\\ there", 1, 0, 2, 0.5)');
    expect(text(indBlock(inds.find((i) => i.id === "rsi")!))).toBe("rsi(14)");
    expect(text(indBlock(inds.find((i) => i.id === "macd")!, "signal"))).toBe("macd(12, 26, 9).signal");
    expect(text(indBlock(inds.find((i) => i.id === "macd")!, "macd"))).toBe("macd(12, 26, 9)"); // the first output is the default
  });

  it("emits every function block", () => {
    const calls: [FnId, string][] = [["abs", "Abs(C)"], ["sqrt", "Sqrt(C)"], ["log", "Log(C)"], ["exp", "Exp(C)"], ["ma", "MA(C, 14)"], ["ema", "EMA(C, 14)"],
      ["wma", "WMA(C, 14)"], ["wilders", "Wilders(C, 14)"], ["stdev", "StDev(C, 14)"], ["sum", "Sum(C, 14)"], ["hhv", "HHV(C, 14)"], ["llv", "LLV(C, 14)"],
      ["roc", "ROC(C, 14)"], ["change", "Change(C, 14)"], ["linreg", "LinRegSlope(C, 14)"]];
    for (const [f, out] of calls) expect(text(fn(f, C, num(14)))).toBe(out);
    expect(text(fn("ama", C, num(0.1)))).toBe("AMA(C, 0.1)");
    expect(text(fn("max", C, O))).toBe("Max(C, O)");
    expect(text(fn("min", C, O))).toBe("Min(C, O)");
    expect(text(fn("cross_above", C, O))).toBe("Cross(C, O)");
    expect(text(fn("cross_below", C, O))).toBe("Cross(O, C)");
    expect(text(fn("if", fn("gt", C, O), C, O))).toBe("IIf(C > O, C, O)");
    expect(text(fn("barssince", fn("gt", C, O)))).toBe("BarsSince(C > O)");
    expect(text(fn("valuewhen", fn("gt", C, O), C))).toBe("ValueWhen(C > O, C)");
    expect(text(fn("valuewhen", fn("gt", C, O), C, num(2)))).toBe("ValueWhen(C > O, C, 2)");
    expect(text(fn("nz", C))).toBe("Nz(C)");
    expect(text(fn("nz", C, num(0)))).toBe("Nz(C, 0)");
    const ops: [FnId, string][] = [["add", "C + O"], ["sub", "C - O"], ["mul", "C * O"], ["div", "C / O"], ["pow", "C ^ O"], ["gt", "C > O"], ["lt", "C < O"],
      ["ge", "C >= O"], ["le", "C <= O"], ["eq", "C == O"], ["ne", "C != O"]];
    for (const [f, out] of ops) expect(text(fn(f, C, O))).toBe(out);
    expect(text(fn("and", fn("gt", C, O), { k: "bool", v: true }))).toBe("C > O AND true");
    expect(text(fn("or", fn("gt", C, O), fn("lt", C, O)))).toBe("C > O OR C < O");
    expect(text(fn("not", fn("gt", C, O)))).toBe("NOT (C > O)");
    expect(text(fn("neg", C))).toBe("-C");
    expect(text(ago(C, 3))).toBe("C[3]");
    expect(text(ago(fn("ema", C, num(9)), 1))).toBe("Ref(EMA(C, 9), -1)");
    expect(text(ago(C, 0))).toBe("C[0]");
    expect(text(ago(fn("sub", C, O), 0))).toBe("Ref(C - O, 0)");
  });

  it("adds parentheses only where precedence needs them", () => {
    expect(text(fn("mul", fn("add", C, O), num(2)))).toBe("(C + O) * 2");
    expect(text(fn("add", fn("mul", C, O), num(2)))).toBe("C * O + 2");
    expect(text(fn("sub", C, fn("sub", O, num(1))))).toBe("C - (O - 1)");
    expect(text(fn("sub", fn("sub", C, O), num(1)))).toBe("C - O - 1");
    expect(text(fn("div", C, fn("mul", O, num(2))))).toBe("C / (O * 2)");
    expect(text(fn("pow", fn("add", C, O), num(2)))).toBe("(C + O) ^ 2");
    expect(text(fn("pow", fn("pow", C, O), num(2)))).toBe("(C ^ O) ^ 2");
    expect(text(fn("pow", num(-2), num(2)))).toBe("(-2) ^ 2");
    expect(text(fn("add", C, num(-3)))).toBe("C + (-3)");
    expect(text(fn("neg", fn("add", C, O)))).toBe("-(C + O)");
    expect(text(fn("neg", fn("neg", C)))).toBe("-(-C)");
    expect(text(fn("gt", fn("gt", C, O), num(0)))).toBe("(C > O) > 0");
    expect(text(fn("and", fn("or", fn("gt", C, O), fn("lt", C, O)), { k: "bool", v: true }))).toBe("(C > O OR C < O) AND true");
    expect(text(fn("or", { k: "bool", v: true }, fn("and", fn("gt", C, O), fn("lt", C, O))))).toBe("true OR C > O AND C < O");
    expect(text(fn("and", fn("not", fn("gt", C, O)), fn("lt", C, O)))).toBe("NOT (C > O) AND C < O");
    expect(text(fn("gt", fn("not", { k: "bool", v: true }), num(0)))).toBe("(NOT true) > 0");
    expect(text(fn("mul", fn("ema", fn("add", C, O), num(9)), num(2)))).toBe("EMA(C + O, 9) * 2");
  });

  it("writes named results, plots, placement, holes and spans", () => {
    const m: Blocks = { v: 1, lets: [{ name: "N", expr: param("Periods", 14, 2, 200, 1) }, { name: "Mid", expr: fn("ma", C, ref("N")) }],
      plots: [{ name: 'Up "band"', onPrice: true, expr: fn("add", ref("Mid"), null) }, { name: "Mid", onPrice: false, expr: ref("Mid") }] };
    const g = generate(m);
    expect(g.text).toBe('N = Param("Periods", 14, 2, 200, 1);\nMid = MA(C, N);\nPlot(Mid + ?, "Up \\"band\\"", "price");\nPlot(Mid, "Mid");');
    expect(g.holes).toEqual(["p0.1"]);
    expect(g.text.slice(...g.spans["l1"])).toBe("MA(C, N)");
    expect(g.text.slice(...g.spans["l1.1"])).toBe("N");
    expect(keyAt(g, { line: 2, col: 13 })).toBe("l1.1"); // "N" inside MA(C, N)
    expect(keyAt(g, { line: 3, col: 12 })).toBe("p0.1"); // the hole
    expect(keyAt(g, { line: 2, col: 1 })).toBe("l1:row");
    expect(keyAt(g, { line: 9, col: 1 })).toBeUndefined();
  });
});

describe("parser", () => {
  it("reads the editor examples and round-trips them through blocks", () => {
    for (const x of FORMULA_EXAMPLES) {
      const m = parseFormula(x.formula, inds);
      const once = generate(m).text;
      expect(generate(parseFormula(once, inds)).text).toBe(once);
      expect(parseFormula(once, inds)).toEqual(m);
    }
    expect(generate(parseFormula(FORMULA_EXAMPLES[0].formula, inds)).text).toBe(
      'Fast = Param("Fast", 12, 2, 50, 1);\nSlow = Param("Slow", 26, 5, 200, 1);\nSig = Param("Signal", 9, 2, 50, 1);\nOsc = EMA(C, Fast) - EMA(C, Slow);\nPlot(Osc, "Osc");\nPlot(EMA(Osc, Sig), "Trigger");');
  });

  it("normalises aliases, case and lookback forms", () => {
    expect(roundTrip("x = sma(close, 5)\nplot(x[2], 'n')".replace("'n'", '"n"'))).toBe('x = MA(C, 5);\nPlot(x[2], "n");');
    expect(roundTrip('Plot(Ref(EMA(C,9), -3) + highest(H, 5) - lowest(l, 5) + mom(c, 2) + rma(C, 3), "a")'))
      .toBe('Plot(Ref(EMA(C, 9), -3) + HHV(H, 5) - LLV(L, 5) + Change(C, 2) + Wilders(C, 3), "a")'.replace(/\)$/, ");"));
    expect(roundTrip('Plot(IIf(C > O && !(C < 2) || false, Open, -High), "b")')).toBe('Plot(IIf(C > O AND NOT (C < 2) OR false, O, -H), "b");');
    expect(roundTrip('Plot(RSI(length=10) + MACD(12, 26, 9).Hist + macd().macd, "c")')).toBe('Plot(rsi(10) + macd(12, 26, 9).hist + macd(12, 26, 9), "c");');
    expect(roundTrip('Plot(-2^2 + -(3) - - C, "d")')).toBe('Plot(-(2 ^ 2) + (-3) - (-C), "d");');
    expect(roundTrip('Plot(Cross(EMA(C, 9), EMA(C, 21)), "e"); Plot(ValueWhen(C > O, C, 2) + Nz(C, 0) + BarsSince(C > O), "f", "price")'))
      .toBe('Plot(Cross(EMA(C, 9), EMA(C, 21)), "e");\nPlot(ValueWhen(C > O, C, 2) + Nz(C, 0) + BarsSince(C > O), "f", "price");');
    expect(roundTrip('Plot(EMA(C,\n  9), "multi-line call") // comment')).toBe('Plot(EMA(C, 9), "multi-line call");');
    expect(roundTrip('Plot(C, "say \\"hi\\"")')).toBe('Plot(C, "say \\"hi\\"");');
  });

  it("round-trips every block kind built by hand", () => {
    const all: BlockNode[] = (Object.keys(FNS) as FnId[]).map((f) => fn(f, ...FNS[f].args.map((a) => (a.type === "condition" ? fn("gt", C, O) : a.type === "series" ? C : num(3)))));
    const wrap = (e: BlockNode, i: number) => (i % 2 ? fn("add", e, fn("mul", ago(ref("P"), 2), indBlock(inds[1]))) : e);
    const models: Blocks[] = [];
    for (let i = 0; i < all.length; i += 8)
      models.push({ v: 1, lets: [{ name: "P", expr: param("P", 3, 1, 9, 1) }], plots: all.slice(i, i + 8).map((e, j) => ({ name: `L${i + j}`, onPrice: j === 0, expr: wrap(e, j) })) });
    expect(models.length).toBeGreaterThan(3);
    for (const x of models) {
      const t = generate(x).text;
      const back = parseFormula(t, inds);
      expect(generate(back).text).toBe(t);
    }
  });

  it("refuses what blocks cannot show, with line and column", () => {
    const err = (src: string) => { try { parseFormula(src, inds); } catch (e) { return (e as FormulaParseError).message; } return "parsed"; };
    expect(err('Plot(FOO(C), "a")')).toBe("line 1, col 6: unknown function FOO");
    expect(err('x = 1;\nx = 2;\nPlot(x, "a")')).toBe("line 2, col 1: x is already defined; a name is assigned once");
    expect(err('Plot(Ref(C, 1), "a")')).toMatch(/line 1, col 13: Ref with a positive offset looks into the future/);
    expect(err('Plot(y, "a")')).toBe("line 1, col 6: unknown name y");
    expect(err('C > O;')).toMatch(/blocks can show only/);
    expect(err('x = C;')).toBe("line 1, col 7: the formula has no Plot(…)");
    expect(err('Plot(C, "a", "pane")')).toMatch(/third argument can only be "price"/);
    expect(err('Plot(C $ 2, "a")')).toBe("line 1, col 8: unexpected character “$”");
    expect(err('Plot(C[2001], "a")')).toMatch(/bars ago must be a whole number/);
    expect(err('Plot(EMA(C), "a")')).toBe("line 1, col 6: EMA takes 2 arguments");
    expect(err('Plot(C.x, "a")')).toMatch(/only a catalogue indicator has outputs/);
    expect(err('Plot(rsi(14).nope, "a")')).toMatch(/has no output “nope”/);
    expect(err('Plot(Param("x", 1), "a")')).toMatch(/Param needs a label and four numbers/);
    expect(err('C = 1; Plot(C, "a")')).toMatch(/C is a built-in name/);
    expect(err('Plot(C, "a)')).toMatch(/no closing quote/);
  });
});

describe("edits and checks", () => {
  it("refuses wrong types with a reason and accepts right ones", () => {
    const m = plot(fn("ema", null, null));
    const bad = place(m, "p0.1", C, inds);
    expect(bad).toEqual({ ok: false, reason: "The length slot needs a whole number (like 14, or a whole-number Parameter); Close gives a series." });
    expect(place(m, "p0.1", num(2.5), inds).ok).toBe(false);
    expect(place(m, "p0.1", param("N", 2, 1, 5, 0.5), inds).ok).toBe(false);
    expect(place(m, "p0.1", param("N"), inds).ok).toBe(true);
    expect(place(m, "p0.0", fn("gt", C, O), inds).ok).toBe(true); // booleans are 1/0 series
    const cond = plot(fn("and", null, null));
    expect(place(cond, "p0.0", C, inds)).toMatchObject({ ok: false, reason: expect.stringContaining("needs a condition") });
    const placed = place(m, "p0.1", num(9), inds);
    expect(placed.ok && generate(placed.model).text).toBe('Plot(EMA(?, 9), "X");');
  });

  it("moves blocks, refuses moving into itself, guards results used before their definition", () => {
    const m: Blocks = { v: 1, lets: [{ name: "A", expr: num(5) }, { name: "B", expr: C }], plots: [{ name: "X", onPrice: false, expr: fn("ema", C, ref("A")) }] };
    const moved = place(m, "l1", getAt(m, "p0.0")!, inds, "p0.0");
    expect(moved.ok && moved.model.lets[1].expr).toEqual(C);
    expect(moved.ok && getAt(moved.model, "p0.0")).toBeNull();
    expect(place(m, "p0.0", getAt(m, "p0")!, inds, "p0")).toEqual({ ok: false, reason: "A block cannot go inside itself." });
    expect(place(m, "l0", ref("B"), inds)).toMatchObject({ ok: false, reason: expect.stringContaining("defined below this row") });
    expect(place(m, "l1", ref("A"), inds).ok).toBe(true);
    // changing A into a series would break EMA's length slot, which uses A: refused
    expect(place(m, "l0", C, inds)).toMatchObject({ ok: false, reason: expect.stringContaining("length needs a whole number") });
  });

  it("renames results everywhere and reports problems", () => {
    const m: Blocks = { v: 1, lets: [{ name: "A", expr: num(5) }], plots: [{ name: "X", onPrice: false, expr: fn("ema", C, ref("a")) }] };
    expect(generate(renameLet(m, 0, "Len")).text).toBe('Len = 5;\nPlot(EMA(C, Len), "X");');
    const p = problems({ v: 1, lets: [{ name: "1x", expr: null }, { name: "close", expr: C }, { name: "Q", expr: param("A", 5, 9, 1, 0) }],
      plots: [{ name: "", onPrice: false, expr: ref("zz") }, { name: "", onPrice: false, expr: fn("valuewhen", fn("gt", C, O), C) }] }, inds);
    const msgs = p.map((x) => `${x.key}|${x.kind}`);
    expect(msgs).toEqual(expect.arrayContaining(["l0|empty", "l0:name|other", "l1:name|other", "l2|other", "p0|type", "p0:name|other", "p1:name|other"]));
    expect(msgs).not.toContain("p1.2|empty"); // nth is optional
    expect(problems(starterBlocks(), inds)).toEqual([]);
    expect(problems({ v: 1, lets: [], plots: [] }, inds)[0].message).toBe("Add at least one line to draw.");
    expect(emptyKeys(plot(fn("ema", null, num(3))))).toEqual(["p0.0"]);
  });

  it("reads saved layouts defensively", () => {
    const m = starterBlocks();
    expect(readBlocks(JSON.parse(JSON.stringify(m)))).toEqual(m);
    expect(readBlocks(null)).toBeNull();
    expect(readBlocks({ v: 2, lets: [], plots: [] })).toBeNull();
    expect(readBlocks({ v: 1, lets: [], plots: [{ name: "x", expr: { k: "fn", f: "nope", a: [] } }] })).toBeNull();
    expect(readBlocks({ v: 1, lets: [], plots: [{ name: "x", expr: { k: "fn", f: "ema", a: [null] } }] })).toBeNull();
    expect(readBlocks({ v: 1, lets: [], plots: [{ name: "x", expr: null }] })).toEqual(emptyBlocks().plots[0] && { v: 1, lets: [], plots: [{ name: "x", onPrice: false, expr: null }] });
    expect(setAt(m, "p0.1.1", num(3)).plots[0].expr).toEqual(fn("sub", C, fn("ma", C, num(3))));
  });
});

describe("palette", () => {
  it("lists every group and searches by name, formula name and keyword", () => {
    const items = paletteItems(starterBlocks(), inds);
    expect(new Set(items.map((i) => i.group))).toEqual(new Set(["Price", "Number", "Parameter", "My results", "Indicators", "Math", "Smoothing & windows", "Lookback", "Compare", "Logic", "State"]));
    expect(items.filter((i) => i.group === "My results").map((i) => i.label)).toEqual(["N"]);
    // catalogue ids shadowed by built-in functions (ema) and indicators the formula cannot call are not offered
    expect(items.filter((i) => i.group === "Indicators").map((i) => i.id)).toEqual(["ind:rsi", "ind:macd", "ind:stoch", "ind:obv", "ind:ad_line", "ind:fractals", "ind:doji"]);
    expect(searchPalette(items, "").length).toBe(items.length);
    expect(searchPalette(items, "cross").map((i) => i.label)).toEqual(["crosses above", "crosses below"]);
    expect(searchPalette(items, "HHV").map((i) => i.label)).toEqual(["Highest"]);
    expect(searchPalette(items, "relative strength").map((i) => i.label)).toEqual(["RSI"]);
    expect(searchPalette(items, "oscillators macd").map((i) => i.label)).toEqual(["MACD"]);
    expect(items.find((i) => i.id === "param")!.make()).toEqual(param("Periods2"));
  });
});
