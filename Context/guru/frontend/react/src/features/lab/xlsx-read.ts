/** Reading an uploaded .xlsx in the browser: each sheet's name and its rows of cell text (a date cell is Excel's day number,
 *  which isoDate reads). An .xlsx is a zip of XML files; entries are stored or deflated, which DecompressionStream inflates. */
export interface Sheet { name: string; rows: string[][] }

const u16 = (b: Uint8Array, i: number) => b[i] | (b[i + 1] << 8);
const u32 = (b: Uint8Array, i: number) => (u16(b, i) | (u16(b, i + 2) << 16)) >>> 0;

/** The zip's files by name (contents still compressed until read). */
function entries(b: Uint8Array) {
  let end = b.length - 22;
  while (end >= 0 && u32(b, end) !== 0x06054b50) end--;
  if (end < 0) throw new Error("This is not an .xlsx file. Save it as .xlsx or CSV and try again.");
  const out = new Map<string, { method: number; size: number; at: number }>();
  for (let i = 0, p = u32(b, end + 16); i < u16(b, end + 10); i++) {
    const nameLen = u16(b, p + 28), name = new TextDecoder().decode(b.subarray(p + 46, p + 46 + nameLen));
    out.set(name, { method: u16(b, p + 10), size: u32(b, p + 20), at: u32(b, p + 42) });
    p += 46 + nameLen + u16(b, p + 30) + u16(b, p + 32);
  }
  return out;
}

async function text(b: Uint8Array, files: ReturnType<typeof entries>, name: string): Promise<string | undefined> {
  const f = files.get(name);
  if (!f) return undefined;
  const start = f.at + 30 + u16(b, f.at + 26) + u16(b, f.at + 28), data = b.slice(start, start + f.size);
  if (f.method === 0) return new TextDecoder().decode(data);
  if (f.method !== 8) throw new Error("This .xlsx uses a compression the browser cannot read; save it as CSV instead.");
  const raw = new ReadableStream<Uint8Array>({ start(c) { c.enqueue(data); c.close(); } });
  return new Response(raw.pipeThrough(new DecompressionStream("deflate-raw") as unknown as TransformStream<Uint8Array, Uint8Array>)).text();
}

const xml = (s: string) => new DOMParser().parseFromString(s, "application/xml");
const tags = (d: Document | Element, tag: string) => [...d.getElementsByTagNameNS("*", tag)];
/** "C12" → 2 (column index from 0). */
const column = (ref: string) => [...(ref.match(/^[A-Z]+/)?.[0] ?? "A")].reduce((n, c) => n * 26 + c.charCodeAt(0) - 64, 0) - 1;

export async function readXlsx(buffer: ArrayBuffer): Promise<Sheet[]> {
  const b = new Uint8Array(buffer), files = entries(b);
  const shared = tags(xml((await text(b, files, "xl/sharedStrings.xml")) ?? "<sst/>"), "si").map((si) => tags(si, "t").map((t) => t.textContent ?? "").join(""));
  const book = xml((await text(b, files, "xl/workbook.xml")) ?? "");
  const rels = new Map(tags(xml((await text(b, files, "xl/_rels/workbook.xml.rels")) ?? "<r/>"), "Relationship").map((r) => [r.getAttribute("Id"), r.getAttribute("Target") ?? ""]));
  const sheets: Sheet[] = [];
  for (const s of tags(book, "sheet")) {
    const target = rels.get(s.getAttribute("r:id") ?? [...s.attributes].find((a) => a.localName === "id")?.value ?? "") ?? "";
    const body = await text(b, files, target.startsWith("/") ? target.slice(1) : `xl/${target.replace(/^\.\//, "")}`);
    if (!body) continue;
    const rows = tags(xml(body), "row").map((r) => {
      const row: string[] = [];
      for (const c of tags(r, "c")) {
        const v = tags(c, "v")[0]?.textContent ?? tags(c, "t").map((t) => t.textContent ?? "").join("");
        row[column(c.getAttribute("r") ?? "")] = c.getAttribute("t") === "s" ? shared[Number(v)] ?? "" : v;
      }
      return Array.from(row, (x) => x ?? "");
    });
    sheets.push({ name: s.getAttribute("name") ?? "", rows });
  }
  return sheets;
}
