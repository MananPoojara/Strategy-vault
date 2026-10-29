// The research-template tradesheet: built with the vendored SheetJS, written to xlsx bytes, read back, and every checked
// cached value compared to a hand computation from the fixture below (not to the module's own evaluator).
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import vm from "node:vm";
import { describe, expect, it } from "vitest";
import { buildTsWorkbook, defaultTsFormat, tsFileName, tsSheetName, tsSheetNames, TS_COSTS, type TsFormat, type TsInput } from "../src/features/lab/ts-workbook";

// The min.js is a UMD bundle; under CommonJS it requires ./cpexcel.js, so run it in a bare context where it sets XLSX.
const XLSX = vm.runInNewContext(`${readFileSync(resolve(process.cwd(), "public/vendor/xlsx-js-style-1.2.0.min.js"), "utf8")};XLSX`, {});

const DAY = 864e5, EPOCH = Date.UTC(1899, 11, 30);
const ser = (iso: string) => (Date.parse(`${iso}T00:00:00Z`) - EPOCH) / DAY;
const TRADES: TsInput["trades"] = [
  { dir: 1, entry: "2007-12-03", exit: "2007-12-10", entry_px: 100, exit_px: 104 },
  { dir: -1, entry: "2008-03-03", exit: "2008-03-12", entry_px: 200, exit_px: 190 },
  { dir: 1, entry: "2011-01-03", exit: "2011-01-12", entry_px: 300, exit_px: 291 },
  { dir: -1, entry: "2011-06-01", exit: "2011-06-09", entry_px: 400, exit_px: 396 },
  { dir: 1, entry: "2011-07-01", exit: "2011-07-05", entry_px: 410, exit_px: 415, open: true },
];
// Weekday bars from two days before each trade to two days after it (~45 bars); the open trade's bars end at its "exit",
// which is the last bar, as the caller passes it.
const OHLC: TsInput["ohlc"] = TRADES.flatMap((t) => {
  const out: TsInput["ohlc"] = [];
  for (let d = Date.parse(t.entry) - 2 * DAY; d <= Date.parse(t.exit) + (t.open ? 0 : 2 * DAY); d += DAY) {
    if ([0, 6].includes(new Date(d).getUTCDay())) continue;
    const close = t.entry_px * (1 + 0.02 * Math.sin(out.length + t.entry_px));
    out.push({ date: new Date(d).toISOString().slice(0, 10), open: close * 0.997, high: close * 1.013, low: close * 0.984, close });
  }
  return out;
});
// The open trade is marked at the last bar's close, as the caller passes it.
TRADES[4].exit_px = OHLC[OHLC.length - 1].close;
const RULES: TsInput["rules"] = { timeframe: "Daily", index: "NIFTY 50", period: "2007-12-01 to 2011-06-30", strategy: "Fixture", position: "Long & Short",
  filter: "Without Filter", setup: "Test setup", entry: ["Entry A", "Entry B"], exit: ["Exit A"] };
const input = (format: TsFormat, costs = TS_COSTS): TsInput => ({ format, sheetName: "NF_En'BFR:0", symbol: "NIFTY 50", trades: TRADES, ohlc: OHLC, costs, rules: RULES });

// ---- hand computation of what the templates' formulas give for the fixture ----
// Open trades count like closed ones: exit = the date and price given (the last bar's close).
function expected(costs: typeof TS_COSTS, trades = TRADES) {
  const closed = trades;
  let gc = 100, gp = 100, nc = 100, np = 100;
  const rows = closed.map((t) => {
    const g = t.dir === 1 ? t.exit_px / t.entry_px - 1 : -(t.exit_px / t.entry_px - 1);
    const days = ser(t.exit) - ser(t.entry);
    const net = g - costs.impact - days / 30 * (t.dir === 1 ? costs.rollLong : costs.rollShort);
    gc *= 1 + g; gp = Math.max(gp, gc); nc *= 1 + net; np = Math.max(np, nc);
    const win = OHLC.filter((b) => ser(b.date) > ser(t.entry) && ser(b.date) <= ser(t.exit));
    const mae = t.dir === 1 ? Math.min(...win.map((b) => b.low)) / t.entry_px - 1 : -(Math.max(...win.map((b) => b.high)) / t.entry_px - 1);
    return { t, g, days, net, gc, gdd: gp > gc ? (gc - gp) / gp : 0, nc, ndd: np > nc ? (nc - np) / np : 0, mae };
  });
  const lastExit = Math.max(...closed.map((t) => ser(t.exit)));
  // A block from `from`: its trades, and CAGR from the net CUMM before its first trade to the CUMM at the last exit.
  const block = (from: string) => {
    const i = rows.findIndex((r) => r.t.entry >= from), rs = rows.slice(i), nets = rs.map((r) => r.net);
    const before = i ? rows[i - 1].nc : 100, start = ser(rows[i].t.entry);
    return {
      total: nets.reduce((a, b) => a + b, 0), avg: nets.reduce((a, b) => a + b, 0) / nets.length, trades: nets.length,
      win: nets.filter((n) => n > 0).length / nets.length,
      cagr: (rows[rows.length - 1].nc / before) ** (1 / ((lastExit - start) / 365)) - 1,
      dd: Math.min(0, ...rs.map((r) => r.ndd)), maxMae: Math.min(...rs.map((r) => r.mae)),
    };
  };
  const yoy = (y: number) => rows.filter((r) => r.t.exit.startsWith(String(y))).reduce((a, r) => a + r.net, 0);
  return { rows, block, yoy };
}

const roundTrip = (wb: any) => XLSX.read(XLSX.write(wb, { bookType: "xlsx", type: "array", cellStyles: true }), { type: "array", cellFormula: true, cellStyles: true });
const SHEET = tsSheetName("NF_En'BFR:0");

describe("tradesheet in the research template layout", () => {
  it("picks the format by start date and sanitises the trade sheet name", () => {
    expect(defaultTsFormat("2000-01-03")).toBe("2000");
    expect(defaultTsFormat("2007-12-31")).toBe("2000");
    expect(defaultTsFormat("2008-01-01")).toBe("2008");
    expect(SHEET).toBe("NF_En BFR 0"); // apostrophe and colon removed
    expect(tsSheetName("'" + "x".repeat(40) + "'")).toHaveLength(31);
    expect(tsSheetName("Rules")).toBe("TS Rules");
    expect(tsFileName(input("2008"))).toBe(`TS_${SHEET}.xlsx`);
  });

  for (const format of ["2008", "2000"] as TsFormat[]) {
    describe(`format ${format}`, () => {
      const costs = format === "2008" ? TS_COSTS : { impact: 0.002, rollLong: 0.012, rollShort: -0.004 };
      const wb = roundTrip(buildTsWorkbook(XLSX, input(format, costs)));
      const ts = wb.Sheets[SHEET], sum = wb.Sheets["2008-2011 Summary"], yoy = wb.Sheets["YOY Summary"];
      const ex = expected(costs);
      // Column letters differ: 2008 puts MAE/MFE at L/M and CUMM from N; 2000 puts CUMM at L and MAE at W.
      const col = format === "2008" ? { cumm: "N", dd: "Q", mae: "L", ndd: "V" } : { cumm: "L", dd: "O", mae: "W", ndd: "T" };
      const first = format === "2008" ? 23 : 21;

      it("has the template's sheets in order, with Rules filled", () => {
        expect(wb.SheetNames).toEqual(["All Index Explore", "All Sheets Name", "Rules", "2008-2011 Summary", "YOY Summary", SHEET]);
        const rules = wb.Sheets.Rules, at = (r: string) => rules[r]?.v;
        expect([at("B2"), at("C2"), at("B3"), at("C3"), at("B4"), at("C4"), at("B5"), at("C5"), at("B6"), at("B7")])
          .toEqual(["Timeframe:", "Daily", "Index:", "NIFTY 50", "Period:", RULES.period, "AFL Path:", "", "Done By:", "Verified By:"]);
        expect([at("B9"), at("C9"), at("C10"), at("C11"), at("C12"), at("B13"), at("C13"), at("C14"), at("B15"), at("C15")])
          .toEqual(["Strategy:", "Fixture", "Long & Short", "Without Filter", "Test setup", "Entry:", "Entry A", "Entry B", "Exit:", "Exit A"]);
        expect(wb.Sheets["All Sheets Name"].B5.v).toBe(SHEET);
        expect(wb.Sheets["All Index Explore"].A1.v).toBe("Ticker");
        expect(wb.Sheets["All Index Explore"][`B${OHLC.length + 1}`].v).toBe(ser(OHLC[OHLC.length - 1].date));
      });

      it("writes the template's formula text, as array formulas exactly where the template has them", () => {
        const T = format === "2008" ? {
          J1: "INDEX(C23:C1048576,MATCH(I1,X23:X1048576,0),1)",
          I3: 'SUM(IF(H$1<=C23:C5000,G23:G5000,""))',
          B14: "(INDEX($N$22:$N$5001,MATCH(MAX($E$22:$E$5000),$E$22:$E$5000,0),1)/100)^(1/(((MAX(E22:E5000)-MIN(C22:C5000))/365)))-1",
          L14: "(INDEX(S22:S5000,MATCH(L1,E22:E5000,0),1)/INDEX(S22:S5000,MATCH(J1,C22:C5000,0)-1,1))^(1/(((MAX(E22:E5000)-J1)/365)))-1",
          R4: 'COUNT(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$G$23:$G$5000,""))',
          G23: 'IF((B23="Long")+(B23="Open Long"),(F23/D23-1),-(F23/D23-1))',
          G24: 'IF((B24="Long")+(B24="Open Long"),(F24/D24-1),-(F24/D24-1))',
          J24: 'IF((B24="Long")+(B24="Open Long"),H24/30*D$1,H24/30*E$1)',
          L24: `IF((B24="Long")+(B24="Open Long"),MIN(IF((A24='All Index Explore'!$A$2:$A$61022)*(C24<'All Index Explore'!$B$2:$B$61022)*(E24>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$E$2:$E$61022,""))/D24-1,-(MAX(IF((A24='All Index Explore'!$A$2:$A$61022)*(C24<'All Index Explore'!$B$2:$B$61022)*(E24>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$D$2:$D$61022,""))/D24-1))`,
          N24: "N23*(1+G24)",
          AE24: 'IFERROR(IF(SUMIF($W$23:$W$1075,AD24,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD24,$K$23:$K$1075)),"")',
        } : {
          J1: "INDEX(C21:C1048576,MATCH(I1,V21:V1048575,0),1)",
          I3: 'SUM(IF(H$1<=C21:C1048576,G21:G1048576,""))',
          I14: "(INDEX(L20:L1048573,MATCH(L1,E20:E1048573,0),1)/INDEX(L20:L1048573,MATCH(J1,C20:C1048573,0)-1,1))^(1/(((MAX(E20:E1048573)-J1)/365)))-1",
          E17: "MIN($W$21:$W$374)",
          L17: 'MIN(IF(H1<=$E$21:$E$374,$W$21:$W$374,""))',
          Z4: 'COUNT(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$G$21:$G$1048576,""))',
          G22: 'IF(B22="Long",(F22/D22-1),-(F22/D22-1))',
          O22: "IFERROR(N22/M22,0)",
          W22: `IF(B22="Long",MIN(IF((A22='All Index Explore'!$A$2:$A$52322)*(C22<'All Index Explore'!$B$2:$B$52322)*(E22>='All Index Explore'!$B$2:$B$52322),'All Index Explore'!$E$2:$E$52322,""))/D22-1,-(MAX(IF((A22='All Index Explore'!$A$2:$A$52322)*(C22<'All Index Explore'!$B$2:$B$52322)*(E22>='All Index Explore'!$B$2:$B$52322),'All Index Explore'!$D$2:$D$52322,""))/D22-1))`,
          AA22: 'IFERROR(IF(SUMIF($U$21:$U$250,Z22,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z22,$K$21:$K$250)),"")',
        };
        for (const [ref, f] of Object.entries(T)) expect(ts[ref]?.f, ref).toBe(f);
        const arrays = format === "2008" ? { J1: true, I3: true, B14: true, L14: true, R4: true, G23: true, G24: false, L24: true, N24: false, AE24: true, B3: false }
          : { J1: false, I3: true, I14: false, L17: true, Z4: true, G22: false, W22: true, AA22: false, B3: false };
        for (const [ref, isArray] of Object.entries(arrays)) expect(ts[ref].F, ref).toBe(isArray ? ref : undefined);
        const S = format === "2008"
          ? { C4: `INDIRECT("'"&$C$3&"'!B"&ROW()+B4)`, E4: `IFERROR(IF($C4="","",INDIRECT("'" & $C4 & "'!E3")),"")`, Q5: `IFERROR(IF($C5="","",INDIRECT("'" & $C5 & "'!L14")),"")` }
          : { C4: `INDIRECT("'"&$C$3&"'!B"&ROW()+B4)`, E4: `IF($C5="","",INDIRECT("'" & $C5 & "'!E3"))`, T4: `IF($C5="","",INDIRECT("'" & $C5 & "'!e17"))`, Q6: `IF($C4="","",INDIRECT("'" & $C4 & "'!S14"))` };
        for (const [ref, f] of Object.entries(S)) expect(sum[ref]?.f, ref).toBe(f);
        expect(yoy.C9.f).toBe(format === "2008" ? `IFERROR(INDIRECT("'"&C$4&"'!AE"&ROW()+17),"")` : `INDIRECT("'"&C$4&"'!aa"&ROW()+15)`);
      });

      it("formats every date cell dd-mm-yyyy", () => {
        const dates = [ts[`C${first}`], ts[`E${first}`], ts[`C${first + 4}`], ts[`E${first + 4}`], ts.H1, format === "2008" ? ts.P4 : ts.X4,
          format === "2000" ? ts.O1 : ts.Q4, wb.Sheets["All Index Explore"].B2, wb.Sheets["All Index Explore"][`B${OHLC.length + 1}`]];
        for (const c of dates) expect(c.z).toBe("dd-mm-yyyy");
      });

      it("takes the costs from the input and keeps the period starts", () => {
        expect([ts.C1.v, ts.D1.v, ts.E1.v]).toEqual([costs.impact, costs.rollLong, costs.rollShort]);
        if (format === "2008") expect(ts.H1.v).toBe(ser("2011-01-01"));
        else expect([ts.H1.v, ts.O1.v]).toEqual([ser("2008-01-01"), ser("2011-01-01")]);
      });

      it("caches trade-row values that match a hand computation, the open last trade as a normal row", () => {
        const open = first + 4, mark = OHLC[OHLC.length - 1];
        expect([ts[`A${open}`].v, ts[`B${open}`].v, ts[`E${open}`].v, ts[`F${open}`].v]).toEqual(["NIFTY 50", "Long", ser(mark.date), mark.close]);
        expect(ts[`A${open + 1}`]).toBeUndefined();
        expect(ex.rows).toHaveLength(5);
        ex.rows.forEach((x, i) => {
          const r = first + i;
          expect(ts[`B${r}`].v).toBe(x.t.dir === 1 ? "Long" : "Short");
          expect(ts[`C${r}`].v).toBe(ser(x.t.entry));
          expect(ts[`G${r}`].v).toBeCloseTo(x.g, 12);
          expect(ts[`H${r}`].v).toBe(x.days);
          expect(ts[`K${r}`].v).toBeCloseTo(x.net, 12);
          expect(ts[`${col.cumm}${r}`].v).toBeCloseTo(x.gc, 10);
          expect(ts[`${col.dd}${r}`].v).toBeCloseTo(x.gdd, 12);
          expect(ts[`${col.ndd}${r}`].v).toBeCloseTo(x.ndd, 12);
          expect(ts[`${col.mae}${r}`].v).toBeCloseTo(x.mae, 12);
        });
      });

      it("caches the summary blocks and a YOY value that match a hand computation", () => {
        const all = ex.block("0000"), b2008 = ex.block("2008-01-01"), b2011 = ex.block("2011-01-01");
        // Overall CAGR: E14 runs from the first entry, not from the CUMM before it.
        const firstEntry = ser(TRADES[0].entry), lastExit = Math.max(...TRADES.map((t) => ser(t.exit)));
        const overallCagr = (ex.rows[ex.rows.length - 1].nc / 100) ** (1 / ((lastExit - firstEntry) / 365)) - 1;
        const check = (row: number, b: ReturnType<typeof ex.block>, cagr = b.cagr) => {
          expect(sum[`E${row}`].v, `E${row}`).toBeCloseTo(b.total, 12);
          expect(sum[`F${row}`].v).toBeCloseTo(b.avg, 12);
          expect(sum[`H${row}`].v).toBe(b.trades);
          expect(sum[`I${row}`].v).toBeCloseTo(b.win, 12);
          expect(sum[`Q${row}`].v, `Q${row}`).toBeCloseTo(cagr, 10);
          expect(sum[`R${row}`].v, `R${row}`).toBeCloseTo(b.dd, 12);
        };
        if (format === "2008") {
          expect([sum.D4.v, sum.D5.v, sum.C4.v, sum.C5.v]).toEqual([2008, 2011, SHEET, SHEET]);
          check(4, all, overallCagr);
          check(5, b2011);
          expect(sum.U4.v).toBeCloseTo(all.maxMae, 12); // Max MAE (B19)
          expect([yoy.B9.v, yoy.C4.v]).toEqual([2011, SHEET]);
        } else {
          expect([sum.D4.v, sum.D5.v, sum.D6.v, sum.C6.v]).toEqual([2000, 2008, 2011, SHEET]);
          check(4, all, overallCagr);
          check(5, b2008);
          check(6, b2011);
          expect(sum.T4.v).toBeCloseTo(all.maxMae, 12); // Max MAE (E17)
          expect(sum.T6.v).toBeCloseTo(b2011.maxMae, 12); // S17, from exits on/after O1
          expect([yoy.B9.v, yoy.C4.v]).toEqual([2011, SHEET]);
        }
        expect(yoy.C9.v).toBeCloseTo(ex.yoy(2011), 12);
        expect(yoy.C6.v).toBeCloseTo(ex.yoy(2008), 12);
      });
    });
  }

  describe("several trade sheets in one book", () => {
    // Same entry, three exit variants; the last two names collide after sanitising.
    const SHEETS: NonNullable<TsInput["sheets"]> = [
      { sheetName: "NF_En_BFR_0d_EX_Ncandle-9", trades: TRADES, exitLabel: "N Candles (9)" },
      { sheetName: "NF_En_BFR_0 day_exit_atr 5:1", trades: [TRADES[1], { ...TRADES[2], exit_px: 309 }], exitLabel: "ATR (5,1)" },
      { sheetName: "NF_En_BFR_0 day_exit_atr 5?1", trades: [TRADES[0], TRADES[2], { ...TRADES[3], exit_px: 390 }], exitLabel: "ATR (5,1) close" },
    ];
    const NAMES = ["NF_En_BFR_0d_EX_Ncandle-9", "NF_En_BFR_0 day_exit_atr 5 1", "NF_En_BFR_0 day_exit_atr 5 (2)"];

    it("de-duplicates sanitised names with a short suffix, within 31 characters", () => {
      expect(tsSheetNames(SHEETS.map((s) => s.sheetName))).toEqual(NAMES);
      expect(tsSheetNames(["a", "A", "a"])).toEqual(["a", "A (2)", "a (3)"]);
      expect(tsSheetNames(["x".repeat(40), "x".repeat(35)])).toEqual(["x".repeat(31), `${"x".repeat(27)} (2)`]);
    });

    it("refuses more than 20 trade sheets", () => {
      const many = Array.from({ length: 21 }, (_, i) => ({ sheetName: `S${i}`, trades: TRADES }));
      expect(() => buildTsWorkbook(XLSX, { ...input("2008"), sheets: many })).toThrow(/at most 20 trade sheets/);
      expect(() => buildTsWorkbook(XLSX, { ...input("2008"), sheets: many.slice(0, 20) })).not.toThrow();
    });

    for (const format of ["2008", "2000"] as TsFormat[]) {
      it(`format ${format}: lists, summarises and YOYs every sheet`, () => {
        const wb = roundTrip(buildTsWorkbook(XLSX, { ...input(format), sheets: SHEETS }));
        const sum = wb.Sheets["2008-2011 Summary"], yoy = wb.Sheets["YOY Summary"], names = wb.Sheets["All Sheets Name"];
        expect(wb.SheetNames).toEqual(["All Index Explore", "All Sheets Name", "Rules", "2008-2011 Summary", "YOY Summary", ...NAMES]);
        expect([names.B5.v, names.B6.v, names.B7.v, names.B8]).toEqual([...NAMES, undefined]);
        // Rules: one Exit line per sheet, "name: exit".
        const rules = wb.Sheets.Rules;
        expect([rules.B15.v, rules.C15.v, rules.C16.v, rules.C17.v]).toEqual(["Exit:", ...NAMES.map((n, i) => `${n}: ${SHEETS[i].exitLabel}`)]);
        expect(wb.Sheets[NAMES[1]][format === "2008" ? "P11" : "Y11"].v).toBe("ATR (5,1)"); // the sheet's own Exit line

        const size = format === "2008" ? 2 : 3;
        NAMES.forEach((name, i) => {
          const row = 4 + size * i, ex = expected(TS_COSTS, SHEETS[i].trades), all = ex.block("0000");
          const tsRow = format === "2008" ? row : row + 1; // 2000's first block row reads the name from the row below
          expect(sum[`B${row}`].v).toBe(i + 1);
          expect(sum[`C${row}`].f).toBe(`INDIRECT("'"&$C$3&"'!B"&ROW()+B${row}${i ? `-${size * i}` : ""})`);
          expect(sum[`C${row}`].v).toBe(name);
          expect(sum[`E${row}`].f).toBe(format === "2008" ? `IFERROR(IF($C${tsRow}="","",INDIRECT("'" & $C${tsRow} & "'!E3")),"")` : `IF($C${tsRow}="","",INDIRECT("'" & $C${tsRow} & "'!E3"))`);
          expect(sum[`E${row}`].v, `E${row}`).toBeCloseTo(all.total, 12);
          expect(sum[`H${row}`].v).toBe(all.trades);
          // The 2011 row of each block (last row of the block) reads the 2011 period.
          const r2011 = row + size - 1, b2011 = ex.block("2011-01-01");
          expect(sum[`D${r2011}`].v).toBe(2011);
          expect(sum[`E${r2011}`].v, `E${r2011}`).toBeCloseTo(b2011.total, 12);
          // YOY: one column per sheet, C, D, E.
          const col = String.fromCharCode(67 + i);
          expect(yoy[`${col}2`].v).toBe(i + 1);
          expect(yoy[`${col}4`].v).toBe(name);
          expect(yoy[`${col}4`].f).toBe(format === "2008" ? `IFERROR(INDIRECT("'"&$C$3&"'!B"&ROW()+${col}2),"")` : `INDIRECT("'"&$C$3&"'!B"&ROW()+${col}2)`);
          expect(yoy[`${col}9`].f).toBe(format === "2008" ? `IFERROR(INDIRECT("'"&${col}$4&"'!AE"&ROW()+17),"")` : `INDIRECT("'"&${col}$4&"'!aa"&ROW()+15)`);
          expect(yoy[`${col}9`].v, `${col}9`).toBeCloseTo(ex.yoy(2011), 12);
        });
        expect(sum[`B${4 + size * 3}`]).toBeUndefined();
        expect(yoy.F4).toBeUndefined();
      });
    }
  });
});
