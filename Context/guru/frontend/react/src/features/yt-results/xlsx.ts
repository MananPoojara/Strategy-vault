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
export const col = (i: number): string => (i < 26 ? "" : col(Math.floor(i / 26) - 1)) + String.fromCharCode(65 + (i % 26));
/** Excel sheet names: at most 31 characters, none of []:*?/\ , unique in the workbook. */
export function sheetName(name: string, taken: Set<string>): string {
  const base = name.replace(/[[\]:*?/\\]/g, " ").trim().slice(0, 31) || "Sheet";
  let s = base;
  for (let k = 2; taken.has(s.toLowerCase()); k++) s = `${base.slice(0, 31 - String(k).length - 1)} ${k}`;
  taken.add(s.toLowerCase());
  return s;
}

// Look (the team's reference workbook, "NIFTY_M_S_FUT_BUY_MC100_…patchwise.xlsx"): Calibri 10; header white bold on slate,
// centred and wrapped; titles white bold on navy; Rules labels bold on pale blue; thin light-blue borders; every other data
// row shaded; columns sized to their content. Style ids (cellXfs below): 0 plain, 1 header, 2 title, 3 label,
// 4 + band × 5 + kind for data (kind 0 text, 1 integer, 2 decimal, 3 percent, 4 integer of 10,000 or more with thousands
// separators, so years and counts stay plain, 5 date dd-mm-yyyy; band 1 = shaded row).
const HEADER = 1, TITLE = 2, LABEL = 3, FORM_VALUE = 16; // data cells centre; a Rules value reads left, beside its label
type Kind = 0 | 1 | 2 | 3 | 4 | 5;
const dataStyle = (band: number, kind: Kind) => 4 + band * 6 + kind;
const intKind = (n: number): Kind => (Math.abs(n) >= 10000 ? 4 : 1);
/** A cell's Excel value and kind: "1,234.50" → number, "12.5%" → 0.125 as a percent, "05-01-2012" → an Excel date (so Excel's
 *  status bar shows its Average / Min / Max, as in the TS sheet), a formula → decimal, anything else text. */
function parse(v: CellValue | undefined): { xml: (ref: string, s: number) => string; kind: Kind; shown: number } | null {
  if (v === null || v === undefined || v === "") return null;
  if (typeof v === "object") return { xml: (ref, s) => `<c r="${ref}" s="${s}"><f>${esc(v.f)}</f></c>`, kind: 2, shown: 10 };
  const num = (n: number, kind: Kind) => ({ xml: (ref: string, s: number) => `<c r="${ref}" s="${s}"><v>${n}</v></c>`, kind,
    shown: kind === 3 ? `${(n * 100).toFixed(2)}%`.length : kind === 1 ? String(n).length : kind === 4 ? n.toLocaleString("en-US").length : Math.abs(n).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).length + 1 });
  if (typeof v === "number") return Number.isFinite(v) ? num(v, Number.isInteger(v) ? intKind(v) : 2) : null;
  const dmy = /^(\d{2})-(\d{2})-(\d{4})$/.exec(v.trim());
  if (dmy) {
    const [d, m, y] = [Number(dmy[1]), Number(dmy[2]), Number(dmy[3])], ms = Date.UTC(y, m - 1, d), back = new Date(ms);
    if (back.getUTCDate() === d && back.getUTCMonth() === m - 1) return { ...num(ms / 864e5 + 25569, 5), shown: 10 };
  }
  const t = v.trim().replace(/,/g, "");
  if (/^[-+]?\d+(\.\d+)?%$/.test(t)) return num(Number(t.slice(0, -1)) / 100, 3);
  if (/^[-+]?\d+(\.\d+)?([eE][-+]?\d+)?$/.test(t)) return num(Number(t), Number.isInteger(Number(t)) ? intKind(Number(t)) : 2);
  return { xml: (ref, s) => `<c r="${ref}" s="${s}" t="inlineStr"><is><t xml:space="preserve">${esc(v)}</t></is></c>`, kind: 0, shown: v.length };
}
const text = (v: CellValue | undefined) => (v === null || v === undefined ? "" : typeof v === "object" ? "" : String(v));

/** `{ f }`: an Excel formula without the leading "=", computed when the file opens. */
export type CellValue = string | number | null | { f: string };
export interface Sheet { name: string; rows: CellValue[][]; /** an AutoFilter over the header row and its data */ autoFilter?: boolean;
  /** 0-based column indices hidden in Excel (still there, unhide to see them) */ hidden?: number[];
  /** "table" (default): row 1 is the frozen header; "form": label / value rows (a Rules sheet); "blocks": titled tables, each a
   *  one-cell title row followed by its header row (a report). Only the look differs; every value is written as given. */
  layout?: "table" | "form" | "blocks" }

/** The style of each row: header / title / data (with its shading band) / form. */
function roles(s: Sheet): ("header" | "title" | "form" | number)[] {
  const layout = s.layout ?? "table";
  let band = 0, afterTitle = false;
  return s.rows.map((r, i) => {
    const filled = r.filter((v) => text(v) !== "" || (v !== null && typeof v === "object"));
    if (layout === "form") return "form";
    if (layout === "table") return i === 0 ? "header" : (band++ % 2);
    if (!filled.length) { band = 0; return 0; }
    if (filled.length === 1 && typeof r[0] === "string" && r[0] !== "") { afterTitle = true; return "title"; }
    if (afterTitle) { afterTitle = false; band = 0; return "header"; }
    return band++ % 2;
  });
}

function sheetXml(s: Sheet): string {
  const kinds = roles(s), width = Math.max(1, ...s.rows.map((r) => r.length)), hidden = new Set(s.hidden ?? []);
  // widths from content: a data cell's shown length, a header's longest word (headers wrap), within 8–48 characters
  const widths = Array.from({ length: width }, (_, j) => {
    let w = 8;
    s.rows.forEach((r, i) => {
      const v = r[j], p = parse(v);
      if (!p) return;
      if (kinds[i] === "header") w = Math.max(w, ...text(v).split(/\s+/).map((x) => x.length + 3));
      else if (kinds[i] !== "title") w = Math.max(w, p.shown + 3);
    });
    return Math.min(w, kinds.includes("form") && j > 1 ? 70 : 48);
  });
  const lines = (i: number) => Math.max(1, ...s.rows[i].map((v, j) => Math.ceil(text(v).length / Math.max(1, widths[j] - 2))));
  const rowXml = (r: CellValue[], i: number) => {
    const role = kinds[i];
    const ht = role === "header" ? ` ht="${Math.max(22, 14 * lines(i) + 6)}" customHeight="1"` : role === "title" ? ' ht="22" customHeight="1"' : "";
    const cells = r.map((v, j) => {
      const p = parse(v);
      if (!p) return "";
      const ref = `${col(j)}${i + 1}`;
      if (role === "header") return p.xml(ref, HEADER);
      if (role === "title") return p.xml(ref, TITLE);
      if (role === "form") return p.xml(ref, p.kind === 0 && /:\s*$/.test(text(v)) ? LABEL : p.kind === 0 ? FORM_VALUE : dataStyle(0, p.kind));
      return p.xml(ref, dataStyle(role, p.kind));
    }).join("");
    return `<row r="${i + 1}"${ht}>${cells}</row>`;
  };
  const frozen = (s.layout ?? "table") === "table";
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0"${frozen ? "" : ' showGridLines="0"'}>${
    frozen ? '<pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>' : ""}</sheetView></sheetViews><sheetFormatPr defaultRowHeight="18" customHeight="1"/><cols>${
    widths.map((w, j) => `<col min="${j + 1}" max="${j + 1}" width="${w}"${hidden.has(j) ? ' hidden="1"' : ""} customWidth="1"/>`).join("")}</cols><sheetData>${
    s.rows.map(rowXml).join("")}</sheetData>${
    s.autoFilter && s.rows.length ? `<autoFilter ref="A1:${col(width - 1)}${s.rows.length}"/>` : ""}</worksheet>`;
}

const B = '<left style="thin"><color rgb="FFB0C4D8"/></left><right style="thin"><color rgb="FFB0C4D8"/></right><top style="thin"><color rgb="FFB0C4D8"/></top><bottom style="thin"><color rgb="FFB0C4D8"/></bottom><diagonal/>';
const NUMFMT = [0, 1, 4, 10, 3, 164]; // text General, integer 0, decimal #,##0.00, percent 0.00%, large integer #,##0, date dd-mm-yyyy (custom 164)
const STYLES = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="dd-mm-yyyy"/></numFmts><fonts count="4"><font><sz val="10"/><color rgb="FF000000"/><name val="Calibri"/></font><font><b/><sz val="10"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font><font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font><font><b/><sz val="10"/><color rgb="FF2C3E50"/><name val="Calibri"/></font></fonts><fills count="6"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>${
  ["FF34495E", "FF1F3864", "FFF2F6FA", "FFF9FBFD"].map((c) => `<fill><patternFill patternType="solid"><fgColor rgb="${c}"/><bgColor indexed="64"/></patternFill></fill>`).join("")
}</fills><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border>${B}</border></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="17"><xf fontId="0"/><xf fontId="1" fillId="2" borderId="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center" wrapText="1"/></xf><xf fontId="2" fillId="3" applyFont="1" applyFill="1" applyAlignment="1"><alignment vertical="center"/></xf><xf fontId="3" fillId="4" borderId="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment vertical="center"/></xf>${
  [0, 1].flatMap((band) => NUMFMT.map((fmt, k) => `<xf fontId="0" fillId="${band ? 5 : 0}" borderId="1" numFmtId="${fmt}" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"${k === 0 ? ' wrapText="1"' : ""}/></xf>`)).join("")
}<xf fontId="0" borderId="1" applyFont="1" applyBorder="1" applyAlignment="1"><alignment horizontal="left" vertical="center" indent="1" wrapText="1"/></xf></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>`;

/** A workbook: each sheet laid out as its `layout` says (by default a table with a frozen, filtered-ready header row). */
export function xlsx(sheets: Sheet[]): Uint8Array {
  const taken = new Set<string>(), named = sheets.map((s) => ({ ...s, name: sheetName(s.name, taken) }));
  const ws = sheetXml;
  const formulas = sheets.some((s) => s.rows.some((r) => r.some((v) => v !== null && typeof v === "object")));
  const files: ZipFile[] = [
    { name: "[Content_Types].xml", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>${
      named.map((_, i) => `<Override PartName="/xl/worksheets/sheet${i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>`).join("")}</Types>`) },
    { name: "_rels/.rels", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>`) },
    { name: "xl/workbook.xml", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>${
      named.map((s, i) => `<sheet name="${esc(s.name)}" sheetId="${i + 1}" r:id="rId${i + 1}"/>`).join("")}</sheets>${formulas ? '<calcPr fullCalcOnLoad="1"/>' : ""}</workbook>`) },
    { name: "xl/_rels/workbook.xml.rels", data: utf8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">${
      named.map((_, i) => `<Relationship Id="rId${i + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet${i + 1}.xml"/>`).join("")}<Relationship Id="rId${named.length + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>`) },
    { name: "xl/styles.xml", data: utf8(STYLES) },
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
  // released later, not at once: a browser still reading the link when it is released saves an empty or failed file
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}
export const XLSX_TYPE = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
