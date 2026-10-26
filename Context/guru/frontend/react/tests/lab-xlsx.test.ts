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
  expect(text).toContain('<c r="B2" s="2"><v>0.0125</v></c>');
  expect(text).toContain('<c r="C2"><v>1234.5</v></c>');
  expect(text).toContain("&lt;b&gt;");
  const out = process.env.XLSX_SAMPLE_DIR;
  if (out) { const fs = require("node:fs") as typeof import("node:fs"); fs.writeFileSync(`${out}/sample.xlsx`, book); fs.writeFileSync(`${out}/sample.zip`, zip([{ name: "one.xlsx", data: book }, { name: "two.xlsx", data: book }])); }
});
