// Generated from the 2026-10-22 source review of every catalogue entry (title pages in the research share).
// type: what the source is; sameWorkAs: another entry holding the same work; people: credited authors (none for firm-only credits).
import {AUTHOR_FILES, type SourceReviewRow} from '../catalogue/author-files';
// Each author's rows are `sourceReview` in src/catalogue/authors/<id>.ts.
export const sourceReview: Record<string, SourceReviewRow> = Object.fromEntries(AUTHOR_FILES.flatMap(f => Object.entries(f.sourceReview ?? {})));
// Whole research-share trading library (every census entry classified from its first pages, 2026-10-22):
// copies, editions, translations and parts of one work count once; authors are distinct credited people.
export const shareReview = {"asOf": "2026-10-22", "works": 2748, "authors": 1541, "books": 1109, "articles": 672, "courseManuals": 454, "codePackages": 305, "magazinesAndNewsletters": 119, "videoCourses": 59, "otherSources": 30};

/** File-level count of the research share (\\192.168.4.50\research), duplicates removed (same name and size kept once). Source: deliverables/Research-Share-Resource-Count.xlsx. */
export const shareFiles = {"asOf": "2026-10-29", "authors": 3939, "companies": 413, "found": 311994, "duplicates": 95826, "unique": 216168, "books": 15238, "articles": 17188, "papers": 262, "blogs": 25, "audio": 5913, "video": 25843, "other": 151699, "images": 63825, "webPages": 42382, "tradingCode": 5511, "data": 4056, "programs": 3893, "archives": 1932, "text": 3328, "noExtension": 12626, "unidentified": 14146, "sizeTB": 2.17};
