import {sourceReview} from './source-review-data';
/** Library totals from the reviewed source types: a work held twice counts once; authors are distinct credited people. */
export function libraryTotals(bookIds: string[]) {
  const works = bookIds.map(id => sourceReview[id]).filter(r => r && !r.sameWorkAs);
  const books = works.filter(r => r.type === 'book').length;
  return {books, otherSources: works.length - books, authors: new Set(works.flatMap(r => r.people)).size};
}
