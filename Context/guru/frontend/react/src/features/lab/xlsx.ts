// Minimal .xlsx and .zip writers (entries stored, not compressed): tradesheet downloads without a spreadsheet library.

const CRC = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; }
  return t;
})();
export function crc32(b: Uint8Array): number {
  let c = ~0;
  for (const x of b) c = CRC[(c ^ x) & 255] ^ (c >>> 8);
  return ~c >>> 0;
}

const utf8 = (s: string) => new TextEncoder().encode(s);

export interface ZipFile { name: string; data: Uint8Array }
/** A ZIP archive (method 0, UTF-8 names, dated 1980-01-01). */
export function zip(files: ZipFile[]): Uint8Array {
  const parts: Uint8Array[] = [], central: Uint8Array[] = [];
  let offset = 0;
  for (const f of files) {
    const name = utf8(f.name), crc = crc32(f.data), size = f.data.length;
    const head = (sig: number, len: number) => { const b = new Uint8Array(len); const v = new DataView(b.buffer); v.setUint32(0, sig, true); return { b, v }; };
    const local = head(0x04034b50, 30 + name.length);
    local.v.setUint16(4, 20, true); local.v.setUint16(6, 0x0800, true); local.v.setUint16(12, 33, true);
    local.v.setUint32(14, crc, true); local.v.setUint32(18, size, true); local.v.setUint32(22, size, true); local.v.setUint16(26, name.length, true);
    local.b.set(name, 30);
    const dir = head(0x02014b50, 46 + name.length);
    dir.v.setUint16(4, 20, true); dir.v.setUint16(6, 20, true); dir.v.setUint16(8, 0x0800, true); dir.v.setUint16(14, 33, true);
    dir.v.setUint32(16, crc, true); dir.v.setUint32(20, size, true); dir.v.setUint32(24, size, true); dir.v.setUint16(28, name.length, true);
    dir.v.setUint32(42, offset, true); dir.b.set(name, 46);
    parts.push(local.b, f.data); central.push(dir.b);
    offset += local.b.length + size;
  }
  const dirSize = central.reduce((n, b) => n + b.length, 0);
  const end = new Uint8Array(22), v = new DataView(end.buffer);
  v.setUint32(0, 0x06054b50, true); v.setUint16(8, files.length, true); v.setUint16(10, files.length, true);
  v.setUint32(12, dirSize, true); v.setUint32(16, offset, true);
  const all = [...parts, ...central, end], out = new Uint8Array(all.reduce((n, b) => n + b.length, 0));
  let at = 0;
  for (const b of all) { out.set(b, at); at += b.length; }
  return out;
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/g, "");
const col = (i: number): string => (i < 26 ? "" : col(Math.floor(i / 26) - 1)) + String.fromCharCode(65 + (i % 26));
/** Excel sheet names: at most 31 characters, none of []:*?/\ , unique in the workbook. */
export function sheetName(name: string, taken: Set<string>): string {
  const base = name.replace(/[[\]:*?/\\]/g, " ").trim().slice(0, 31) || "Sheet";
  let s = base;
  for (let k = 2; taken.has(s.toLowerCase()); k++) s = `${base.slice(0, 31 - String(k).length - 1)} ${k}`;
  taken.add(s.toLowerCase());
  return s;
}

/** A cell as Excel sees it: "1,234.50" → number, "12.5%" → 0.125 shown as a percent, anything else text. */
function cell(v: string | number | null | undefined, ref: string, header: boolean): string {
  if (v === null || v === undefined || v === "") return "";
  if (typeof v === "number") return Number.isFinite(v) ? `<c r="${ref}"><v>${v}</v></c>` : "";
  const t = v.trim().replace(/,/g, "");
  if (!header && /^[-+]?\d+(\.\d+)?%$/.test(t)) return `<c r="${ref}" s="2"><v>${Number(t.slice(0, -1)) / 100}</v></c>`;
  if (!header && /^[-+]?\d+(\.\d+)?([eE][-+]?\d+)?$/.test(t)) return `<c r="${ref}"><v>${Number(t)}</v></c>`;
  return `<c r="${ref}" t="inlineStr"${header ? ' s="1"' : ""}><is><t xml:space="preserve">${esc(v)}</t></is></c>`;
}

export interface Sheet { name: string; rows: (string | number | null)[][] }
/** A workbook; the first row of each sheet is the bold, frozen header. */
export function xlsx(sheets: Sheet[]): Uint8Array {
  const taken = new Set<string>(), named = sheets.map((s) => ({ ...s, name: sheetName(s.name, taken) }));
  const ws = (s: Sheet) => `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><sheetData>${
    s.rows.map((r, i) => `<row r="${i + 1}">${r.map((v, j) => cell(v, `${col(j)}${i + 1}`, i === 0)).join("")}</row>`).join("")}</sheetData></worksheet>`;
  const files: ZipFile[] = [
    { name: "[Content_Types].xml", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>${
      named.map((_, i) => `<Override PartName="/xl/worksheets/sheet${i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>`).join("")}</Types>`) },
    { name: "_rels/.rels", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>`) },
    { name: "xl/workbook.xml", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>${
      named.map((s, i) => `<sheet name="${esc(s.name)}" sheetId="${i + 1}" r:id="rId${i + 1}"/>`).join("")}</sheets></workbook>`) },
    { name: "xl/_rels/workbook.xml.rels", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">${
      named.map((_, i) => `<Relationship Id="rId${i + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet${i + 1}.xml"/>`).join("")}<Relationship Id="rId${named.length + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>`) },
    { name: "xl/styles.xml", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="10"/><name val="Arial"/></font><font><b/><sz val="10"/><name val="Arial"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="3"><xf fontId="0"/><xf fontId="1" applyFont="1"/><xf fontId="0" numFmtId="10" applyNumberFormat="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>`) },
    ...named.map((s, i) => ({ name: `xl/worksheets/sheet${i + 1}.xml`, data: utf8(ws(s)) })),
  ];
  return zip(files);
}

/** Saves bytes as a file in the browser. */
export function saveBytes(bytes: Uint8Array, name: string, type: string) {
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type }));
  const a = document.createElement("a");
  a.href = url; a.download = name;
  document.body.append(a); a.click(); a.remove();
  URL.revokeObjectURL(url);
}
export const XLSX_TYPE = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
