import {expect, it} from 'vitest';
import {LIB} from '../src/compat/catalogue.js';
import {authorIdentities} from '../src/services/author-identities';
import {sourceReview} from '../src/services/source-review-data';
import {AUTHOR_FILES} from '../src/catalogue/author-files';

it('maps the Star article book ID to its credited author and reviewed article', async () => {
  // @ts-expect-error classic catalogue dependency initializes window.JoeRossEvidence
  await import('../../joe-ross-evidence.js');
  const owners = LIB.filter((author: {books: {id: string}[]}) => author.books.some(book => book.id === 'bstar'));
  expect(owners.map((author: {id: string}) => author.id)).toEqual(['barbarastar']);
  const entry = AUTHOR_FILES.find(file => file.author.id === 'barbarastar');
  expect(entry?.author.books[0]).toMatchObject({id: 'bstar', year: 1993});
  expect(entry?.author.books[0].title).toContain('article · Stocks & Commodities, April 1993, pp. 174–177');
  expect(entry?.author.books[0].blurb).toContain('John Navarte');
  expect(authorIdentities.barbarastar).toEqual(['Barbara Star']);
  expect(sourceReview.bstar).toEqual({type: 'article', people: ['Barbara Star']});
  expect(entry?.profile?.sources).toContain('https://store.traders.com/-v11-c04-oscilla-pdf.html');
});
