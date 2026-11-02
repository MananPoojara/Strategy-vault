import { expect, it } from "vitest";
import { crc32, sheetName, xlsx, zip } from "../src/features/lab/xlsx";

const u8 = (s: string) => new TextEncoder().encode(s);
it("crc32, zip layout and Excel-safe sheet names", () => {
  expect(crc32(u8("123456789"))).toBe(0xcbf43926);
  const z = zip([{ name: "a.txt", data: u8("hi") }, { name: "b/ü.txt", data: u8("there") }]);
  const v = new DataView(z.buffer);
  expect(v.getUint32(0, true)).toBe(0x04034b50);
  expect(v.getUint32(z.length - 22, true)).toBe(0x06054b50);
  expect(v.getUint16(z.length - 22 + 10, true)).toBe(2);
  const taken = new Set<string>();
  expect([sheetName("Exit 1: off / Exit 2: on", taken), sheetName("Exit 1: off / Exit 2: on", taken), sheetName("x".repeat(40), taken)])
    .toEqual(["Exit 1  off   Exit 2  on", "Exit 1  off   Exit 2  on 2", "x".repeat(31)]);
});
it("an xlsx is a zip with one worksheet per sheet; numbers and percents become numbers", () => {
  const book = xlsx([{ name: "Trades", rows: [["Date", "% Profit", "Price"], ["2024-01-02", "1.25%", "1,234.50"], ["2024-01-03", "-0.50%", "<b>"]] }, { name: "Summary", rows: [["a"], [1]] }]);
  const text = new TextDecoder().decode(book);
  expect(text).toContain('<sheet name="Trades" sheetId="1"');
  // style ids (xlsx.ts): 1 header, 4 + band × 5 + kind (0 text, 1 integer, 2 decimal, 3 percent, 4 large integer)
  expect(text).toContain('<c r="A1" s="1" t="inlineStr"><is><t xml:space="preserve">Date</t></is></c>'); // the slate header
  expect(text).toContain('<c r="B2" s="7"><v>0.0125</v></c>'); // a percent, first data row
  expect(text).toContain('<c r="C2" s="6"><v>1234.5</v></c>'); // a decimal
  expect(text).toContain('<c r="B3" s="12"><v>-0.005</v></c>'); // the next row is shaded
  expect(text).toMatch(/<row r="1" ht="\d+" customHeight="1">/); // the header row is tall enough for wrapped names
  expect(text).toContain('<pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>');
  expect(text).toMatch(/<col min="1" max="1" width="1\d" customWidth="1"\/>/); // fitted to "2024-01-02"
  expect(text).toContain('<name val="Calibri"/>');
  expect(text).toContain("&lt;b&gt;");
  const out = process.env.XLSX_SAMPLE_DIR;
  if (out) { const fs = require("node:fs") as typeof import("node:fs"); fs.writeFileSync(`${out}/sample.xlsx`, book); fs.writeFileSync(`${out}/sample.zip`, zip([{ name: "one.xlsx", data: book }, { name: "two.xlsx", data: book }])); }
});
