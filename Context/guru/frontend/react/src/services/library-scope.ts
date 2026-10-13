export type LibraryScope = "trading" | "technical" | "pending";
export interface ScopeRecord {
  kind: string;
  title: string;
  authorId: string;
  source?: {
    files?: { filename?: string }[];
    source_identity?: { title?: string };
  };
}
const finance =
  /\b(trading|trader|traders|stock market|stock trading|stock prices|stocks|equities|forex|foreign exchange|financial markets|financial derivatives|financial engineering|financial risk|quantitative finance|computational finance|algorithmic trading|technical analysis|options trading|option pricing|option strategies|futures trading|commodity trading|portfolio management|portfolio selection|portfolio optimization|investment|investing|investments|money management|position sizing|hedge funds?|day trading|swing trading|market timing|candlestick|ichimoku|tradestation|metastock|amibroker|easylanguage)\b/i;
const technical =
  /\b(matplotlib|matlab|programming|programmer|programmers|javascript|typescript|python|java|linux|unix|kubernetes|docker|react|angular|nodejs|mongodb|postgresql|mysql|oracle database|html|css|c\+\+|csharp|visual basic|software development|software engineering|web development|data structures|computer science|machine learning|deep learning|tensorflow|pytorch|scipy|numpy|pandas|computer networks|network administration|arduino|raspberry pi|game development|cybersecurity|penetration testing)\b/i;
/** Relevance screening, not a content review or a mechanical-strategy classifier.
 * Financial programming books can remain relevant; a plotting/programming guide
 * is supporting material unless its declared title establishes financial context.
 * Unknown titles remain pending rather than being silently called trading books. */
export function libraryScope(
  record: ScopeRecord,
  curatedAuthors: ReadonlySet<string>,
): LibraryScope {
  if (record.kind === "catalogue") return "trading";
  const title = record.source?.source_identity?.title || record.title;
  if (finance.test(title)) return "trading";
  const filenames = (record.source?.files || [])
    .map((file) => file.filename || "")
    .join(" ");
  if (technical.test(title) || technical.test(filenames)) return "technical";
  if (curatedAuthors.has(record.authorId)) return "trading";
  return "pending";
}
export function inLibraryScope(scope: LibraryScope, selected: string) {
  return selected === "all" || scope === selected;
}
