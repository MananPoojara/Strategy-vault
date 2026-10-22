import {describe, expect, it} from 'vitest';
import {libraryTotals} from '../src/services/source-review';
import {sourceReview} from '../src/services/source-review-data';
describe('libraryTotals', () => {
  it('counts a duplicate edition once and credited people once', () => {
    expect(libraryTotals(['bwtc1', 'bwtc2'])).toEqual({books: 1, otherSources: 0, authors: 1});
    expect(libraryTotals(['stc9']).authors).toBe(0);
  });
  it('covers every catalogue entry', async () => {
    (globalThis as any).window ??= globalThis;
    // @ts-expect-error classic script that sets window.JoeRossEvidence
    await import('../../joe-ross-evidence.js');
    const {LIB} = await import('../src/compat/catalogue.js');
    const ids = LIB.flatMap((g: {books: {id: string}[]}) => g.books.map(b => b.id));
    expect(ids.filter(id => !sourceReview[id])).toEqual([]);
  });
});
