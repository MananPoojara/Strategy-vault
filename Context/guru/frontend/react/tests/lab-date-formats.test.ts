import { deflateRawSync } from "node:zlib";
import { expect, it } from "vitest";
import { chosenPhase, dmy, isoDate, parseDateTable, parseSheets, phaseWindows, withCurrentDateSet } from "../src/features/lab/date-sets";
import { readXlsx } from "../src/features/lab/xlsx-read";
import type { DateSet, Spec } from "../src/features/lab/types";

it("reads every usual way of writing a date, numbers day first", () => {
  for (const text of ["21-11-2008", "21/11/2008", "21.11.2008", "21/11/08", "2008-11-21", "2008/11/21", "20081121", "21 Nov 2008", "21-Nov-08",
    "21st November 2008", "Nov 21, 2008", "November 21 2008", "Fri, 21 Nov 2008", "2008-11-21T00:00:00", "21/11/2008 00:00", "39773", "39773.0", "2008-Nov-21"])
    expect(isoDate(text), text).toBe("2008-11-21");
  expect(isoDate("03/04/2010")).toBe("2010-04-03"); // day first, never month first
  for (const bad of ["31/02/2010", "13/13/2010", "Nov 2008", "12345", "hello"]) expect(isoDate(bad), bad).toBeUndefined();
  expect(dmy("2008-11-21")).toBe("21/11/2008");
});

it("a pasted list: any separators and columns, headers skipped, bull / bear words or the chosen tag", () => {
  const p = parseDateTable("Symbol\tType\tEntry\tExit\nNIFTY 50\tLong\t39773\t40543\n26 Sep 2011 – 25 Nov 2011\nbear, 2010-12-31, 2011-09-26", "Bull");
  expect(p.errors).toEqual([]);
  expect(p.windows).toEqual([{ from: "2008-11-21", to: "2010-12-31", label: "Bull" }, { from: "2011-09-26", to: "2011-11-25", label: "Bull" }, { from: "2010-12-31", to: "2011-09-26", label: "Bear" }]);
  expect(parseDateTable("01-01-2008, 31-01-2008, Crash").windows).toEqual([{ from: "2008-01-01", to: "2008-01-31", label: "Crash" }]); // no tag: the name stays
  expect(parseDateTable("21/11/2008 only one date").errors[0]).toContain("Line 1");
});

it("a set's bull or bear periods go to the strategy, and a re-uploaded set keeps that choice", () => {
  const windows = [{ from: "2008-11-21", to: "2010-12-31", label: "Bull" }, { from: "2010-12-31", to: "2011-09-26", label: "Bear" }, { from: "2011-09-26", to: "2011-11-25", label: "Bull" }];
  expect(phaseWindows(windows, "Bear")).toEqual([windows[1]]);
  expect(chosenPhase([["2010-12-31", "2011-09-26"]], windows)).toBe("Bear");
  const set = { id: 5, name: "Base 2", windows: [...windows, { from: "2011-11-25", to: "2012-06-15", label: "Bear" }] } as DateSet;
  const spec = { v: 1, entry: { all: [] }, exit: { any: [] }, date_filter: { set: { id: 5, name: "Base 2" }, windows: [["2010-12-31", "2011-09-26"]] } } as unknown as Spec;
  expect(withCurrentDateSet(spec, [set]).date_filter!.windows).toEqual([["2010-12-31", "2011-09-26"], ["2011-11-25", "2012-06-15"]]);
});

/** A minimal .xlsx: [path, xml] entries zipped with deflate, as Excel writes them. */
function xlsx(files: [string, string][]): ArrayBuffer {
  const parts: Buffer[] = [], central: Buffer[] = [];
  let at = 0;
  for (const [name, text] of files) {
    const raw = deflateRawSync(Buffer.from(text)), n = Buffer.from(name);
    const local = Buffer.alloc(30); local.writeUInt32LE(0x04034b50, 0); local.writeUInt16LE(8, 8); local.writeUInt32LE(raw.length, 18); local.writeUInt16LE(n.length, 26);
    const dir = Buffer.alloc(46); dir.writeUInt32LE(0x02014b50, 0); dir.writeUInt16LE(8, 10); dir.writeUInt32LE(raw.length, 20); dir.writeUInt16LE(n.length, 28); dir.writeUInt32LE(at, 42);
    parts.push(local, n, raw); central.push(dir, n); at += 30 + n.length + raw.length;
  }
  const size = central.reduce((s, b) => s + b.length, 0), end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0); end.writeUInt16LE(files.length, 10); end.writeUInt32LE(size, 12); end.writeUInt32LE(at, 16);
  const all = Buffer.concat([...parts, ...central, end]);
  return all.buffer.slice(all.byteOffset, all.byteOffset + all.length);
}
const sheet = (rows: string[][]) => `<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>${rows.map((r, i) =>
  `<row r="${i + 1}">${r.map((v, j) => /^\d+$/.test(v) ? `<c r="${"ABCD"[j]}${i + 1}"><v>${v}</v></c>` : `<c r="${"ABCD"[j]}${i + 1}" t="inlineStr"><is><t>${v}</t></is></c>`).join("")}</row>`).join("")}</sheetData></worksheet>`;

it("an Excel workbook with a Bull and a Bear sheet becomes one tagged set, in date order", async () => {
  const book = xlsx([
    ["xl/workbook.xml", `<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Base 2 Bull" sheetId="1" r:id="rId1"/><sheet name="Base 2 Bear" sheetId="2" r:id="rId2"/></sheets></workbook>`],
    ["xl/_rels/workbook.xml.rels", `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Target="worksheets/sheet2.xml"/></Relationships>`],
    ["xl/worksheets/sheet1.xml", sheet([["Symbol", "Type", "Entry", "Exit"], ["NIFTY 50", "Long", "39773", "40543"]])],
    ["xl/worksheets/sheet2.xml", sheet([["Symbol", "Type", "Entry", "Exit"], ["NIFTY 50", "Long", "40543", "40812"]])],
  ]);
  const p = parseSheets(await readXlsx(book));
  expect(p.errors).toEqual([]);
  expect(p.windows).toEqual([{ from: "2008-11-21", to: "2010-12-31", label: "Bull" }, { from: "2010-12-31", to: "2011-09-26", label: "Bear" }]);
});
