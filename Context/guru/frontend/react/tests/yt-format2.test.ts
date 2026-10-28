import { describe, expect, test } from "vitest";
import { buildFormat2, buildFormat2Book, format2SheetName, serial, type F2Bars, type F2Trade } from "../src/services/yt-format2";

const rules = { timeframe: "Daily", index: "Nifty 50", aflPath: "x.afl", doneBy: "", verifiedBy: "", strategy: "S", position: "Long", filter: "Without Filter", setup: "NF", entry: "Before 0 days", exit: ["N candles 3"] };
// weekday bars 2005-01-03 .. 2016-12-30, close 100, high 101, low 99
function bars(): F2Bars {
  const d: string[] = [];
  for (let t = Date.UTC(2005, 0, 3); t <= Date.UTC(2016, 11, 30); t += 86400000) { const x = new Date(t); if (x.getUTCDay() % 6) d.push(x.toISOString().slice(0, 10)); }
  return { ticker: "NIFTY 50", d, o: d.map(() => 100), h: d.map(() => 101), l: d.map(() => 99), c: d.map(() => 100) };
}
const trade = (entry: string, exit: string, entryPx: number, exitPx: number, side: "Long" | "Short" = "Long"): F2Trade => ({ symbol: "NIFTY 50", side, entry, entryPx, exit, exitPx });
const sheet = (trades: F2Trade[]) => buildFormat2({ sheetName: "NF_En_BFR_0d_EX_Ncandle-3", trades, bars: bars(), rules });

describe("Format 2 workbook", () => {
  test("sheets and the 2008 / 2011 blocks start at the first trade year after 2007 / 2010", () => {
    const books = sheet([trade("2005-02-28", "2005-03-03", 100, 102), trade("2015-03-19", "2015-03-24", 100, 99), trade("2016-02-29", "2016-03-03", 100, 103)]);
    expect(books.map(s => s.name)).toEqual(["Rules", "2008-2011 Summary", "YOY Summary", "NF_En_BFR_0d_EX_Ncandle-3", "All Index Explore"]);
    const c = books[3].cells;
    // no trade from 2008 to 2014: both blocks start on 1/1/2015 at the 3/19/2015 trade
    expect([c.H1.v, c.I1.v, c.J1.v]).toEqual([serial("2015-01-01"), 2015, serial("2015-03-19")]);
    expect([c.O1.v, c.P1.v, c.Q1.v]).toEqual([serial("2015-01-01"), 2015, serial("2015-03-19")]);
    expect(c.L1.v).toBe(serial("2016-03-03"));
    expect([c.B5.v, c.I5.v, c.P5.v]).toEqual([3, 2, 2]);
    // the summary keeps the 2000 / 2008 / 2011 labels and reads the overall, L and S columns
    const s = books[1].cells;
    expect([s.D4.v, s.D5.v, s.D6.v]).toEqual([2000, 2008, 2011]);
    expect([s.E4.f, s.E5.f, s.E6.f]).toEqual(["'NF_En_BFR_0d_EX_Ncandle-3'!E3", "'NF_En_BFR_0d_EX_Ncandle-3'!L3", "'NF_En_BFR_0d_EX_Ncandle-3'!S3"]);
    expect(s.E5.v).toBeCloseTo(c.L3.v as number, 12);
  });

  test("trade rows: gross, days, costs and net for longs and shorts, with live formulas", () => {
    const c = sheet([trade("2010-01-04", "2010-02-03", 100, 110), trade("2010-03-01", "2010-03-31", 200, 190, "Short")])[3].cells;
    expect([c.A21.v, c.B21.v, c.C21.v, c.D21.v, c.E21.v, c.F21.v]).toEqual(["NIFTY 50", "Long", serial("2010-01-04"), 100, serial("2010-02-03"), 110]);
    expect(c.G21.v).toBeCloseTo(0.1, 12); expect(c.H21.v).toBe(30); expect(c.J21.v).toBeCloseTo(0.01, 12); expect(c.K21.v).toBeCloseTo(0.1 - 0.003 - 0.01, 12);
    expect(c.G22.v).toBeCloseTo(0.05, 12); expect(c.J22.v).toBeCloseTo(-0.005, 12); expect(c.K22.v).toBeCloseTo(0.05 - 0.003 + 0.005, 12);
    expect(c.G21.f).toBe('IF(B21="Long",(F21/D21-1),-(F21/D21-1))');
    expect(c.I3.array).toBe(true);
    // MAE from the bars after entry through exit: long low 99 -> 99/100-1; short high 101 -> -(101/200-1)
    expect(c.W21.v).toBeCloseTo(-0.01, 12); expect(c.W22.v).toBeCloseTo(0.495, 12);
    expect(c.E15.v).toBe(0);
  });

  test("year table and YOY Summary", () => {
    const books = sheet([trade("2010-01-04", "2010-02-03", 100, 110), trade("2012-01-02", "2012-01-05", 100, 95)]);
    const c = books[3].cells, y = books[2].cells;
    expect(c.Z21.v).toBe(2008); expect(c.AA21.v).toBe(""); expect(c.AA23.v).toBeCloseTo(0.1 - 0.003 - 0.01, 12);
    const totalRow = Object.entries(c).find(([, v]) => v.v === "Total Net Return")![0];
    expect(c[totalRow.replace("Z", "AA")].v).toBeCloseTo((c.AA23.v as number) + (c.AA25.v as number), 12);
    expect(y.B6.v).toBe(2008); expect(y.C8.f).toBe("'NF_En_BFR_0d_EX_Ncandle-3'!AA23");
    expect(Object.values(y).some(x => x.v === "Total Net Return")).toBe(true);
  });

  test("errors like the template when there are no trades", () => {
    const c = sheet([])[3].cells;
    expect(c.B4.e).toBe("#DIV/0!"); expect(c.E14.e).toBe("#N/A"); expect(c.J1.e).toBe("#N/A");
  });

  test("a book with several trade sheets: summary blocks, YOY columns, one explore sheet", () => {
    const b = bars(), bn = { ...bars(), ticker: "NIFTYBANK" };
    const book = buildFormat2Book({ runs: [{ sheetName: "NF_A", ticker: "NIFTY 50", trades: [trade("2010-01-04", "2010-02-03", 100, 110)] },
      { sheetName: "BN_A", ticker: "NIFTYBANK", trades: [{ ...trade("2012-01-02", "2012-01-05", 100, 95), symbol: "NIFTYBANK" }] }], bars: [b, bn], rules: { ...rules, index: ["Nifty 50", "Nifty Bank"] } });
    expect(book.map(s => s.name)).toEqual(["Rules", "2008-2011 Summary", "YOY Summary", "NF_A", "BN_A", "All Index Explore"]);
    const s = book[1].cells, y = book[2].cells, r = book[0].cells, ie = book[5].cells;
    expect([s.B4.v, s.C4.v, s.D4.v, s.B7.v, s.C7.v, s.D9.v]).toEqual([1, "NF_A", 2000, 2, "BN_A", 2011]);
    expect(s.H7.f).toBe("'BN_A'!E5"); expect(s.H7.v).toBe(1);
    expect([y.C4.v, y.D4.v, y.D2.v]).toEqual(["NF_A", "BN_A", 2]);
    expect([r.C3.v, r.C4.v]).toEqual(["Nifty 50", "Nifty Bank"]);
    expect(ie[`A${b.d.length + 2}`].v).toBe("NIFTYBANK");
    // MAE formulas range over every index's rows
    expect(String(book[3].cells.W21.f)).toContain(`$A$2:$A$${2 * b.d.length + 1}`);
  });

  test("team-style sheet names", () => {
    expect(format2SheetName("NIFTY 50", "ythol_long", "Ganesh Chaturthi", { days: 3, after: 0, nc_n: 7, atr_on: 0, hsl: 0, st_on: 0 })).toBe("NF_En_BFR_3d_EX_NC-7");
    expect(format2SheetName("NIFTYBANK", "ythol_long", "", { days: 0, after: 0, nc_n: 0, atr_on: 1, atr_n: 5, atr_mult: 2, hsl: 1, st_on: 0 })).toBe("BN_En_BFR_0d_EX_ATR5,2_HSL-L");
    expect(format2SheetName("NIFTYMIDCAP100", "ytmr_x", "Monday Reversal: Type 1", {})).toBe("MD_Monday Reversal- Type 1");
    expect(format2SheetName("NIFTY 50", "ythol_long", "", { days: 3, after: 0, nc_n: 5, atr_on: 1, atr_n: 5, atr_mult: 1, hsl: 0, st_on: 0 })).toBe("NF_En_BFR_3d_EX_NC-5_ATR5,1");
    expect(format2SheetName("NIFTY 50", "ythol_long", "", { days: 3, after: 0, nc_n: 5, atr_on: 1, atr_n: 5, atr_mult: 1, hsl: 0, st_on: 1, st_factor: 2, st_running: 0 })).toBe("NF_BFR_3d_EX_NC5_ATR5,1_STR2F");
    expect(format2SheetName("NIFTY 50", "ythol_long", "", { days: 3, after: 1, nc_n: 15, atr_on: 1, atr_n: 100, atr_mult: 4, hsl: 2, st_on: 1, st_factor: 4, st_running: 1 }).length).toBeLessThanOrEqual(31);
  });
});
