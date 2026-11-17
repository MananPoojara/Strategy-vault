// Generated from the 2026-10-22 source review of every catalogue entry (title pages in the research share).
// type: what the source is; sameWorkAs: another entry holding the same work; people: credited authors (none for firm-only credits).
import {AUTHOR_FILES, type SourceReviewRow} from '../catalogue/author-files';
// Each author's rows are `sourceReview` in src/catalogue/authors/<id>.ts.
export const sourceReview: Record<string, SourceReviewRow> = Object.fromEntries(AUTHOR_FILES.flatMap(f => Object.entries(f.sourceReview ?? {})));
// Whole research-share trading library: the 2026-10-22 source review (every census entry classified from its first pages) plus,
// on 2026-11-03, the 30 Oct title census, the Stocks & Commodities articles and the folders and code packages found by the full
// share scan (authors/titles-1030/S1w/recount.py, same merge rules): copies, editions, translations and parts of one work count
// once; authors are distinct credited people; firm-only credits are not people; S&C articles count as articles. Round 2 (same day):
// non-trading titles dropped after reading their first pages, uncredited copies merged by title, name typos merged where the title page confirms.
// 2026-11-11: + Aseem Singhal, 51 Trading Strategies (share folder of page images, missed by the 3 Nov count; R4 #292); the 11 Nov full
// re-list of the share (313,477 files) found no other change.
// 2026-11-16: re-checked against the 16 Nov full listing of the share (313,477 files after leaving out folders and Thumbs.db/desktop.ini): the only
// path not in the 29 Oct file inventory is a PDF in the 51 Trading Strategies folder, already inside the 11 Nov count of 313,477; no work, author or book was added or removed, counts unchanged.
export const shareReview = {"asOf": "2026-11-16", "works": 10325, "authors": 2686, "books": 1780, "articles": 5850, "courseManuals": 715, "codePackages": 1393, "magazinesAndNewsletters": 200, "videoCourses": 278, "otherSources": 109};

/** File-level count of the research share (\\192.168.4.50\research), duplicates removed (same name and size kept once). Source: Research-Share-Resource-Count-2026-11-16.xlsx (same rules as the 29 Oct workbook, reproduced exactly on the 29 Oct inventory before use).
 * 16 Nov: +1 file (51 Trading Strategies PDF, counted as a book); authors and companies keep the 29 Oct census (the new file has no author in its name). sizeTB is decimal (2,166.57 GB). */
export const shareFiles = {"asOf": "2026-11-16", "authors": 3939, "companies": 413, "found": 311995, "duplicates": 95826, "unique": 216169, "books": 15239, "articles": 17188, "papers": 262, "blogs": 25, "audio": 5913, "video": 25843, "other": 151699, "images": 63825, "webPages": 42382, "tradingCode": 5511, "data": 4056, "programs": 3893, "archives": 1932, "text": 3328, "noExtension": 12626, "unidentified": 14146, "sizeTB": 2.17};
