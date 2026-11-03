import {expect, it} from 'vitest';
import {activeFilters, authorStatus, CLEAR_ALL, facetAuthors, type AuthorFacts} from '../src/services/author-filters';

const facts: AuthorFacts[] = [
  {id: 'connors', status: 'backtested', type: 'person'},
  {id: 'stad', status: 'backtested', type: 'collection'},
  {id: 'perl', status: 'rules', type: 'person'},
  {id: 'tharp', status: 'reading', type: 'person'},
  {id: 'mags', status: 'reading', type: 'collection'},
];

it('reads status like the card: measured backtest, else catalogued rules, else reading only', () => {
  expect(authorStatus({strategies: 10, backtested: 1})).toBe('backtested');
  expect(authorStatus({strategies: 3, backtested: 0})).toBe('rules');
  expect(authorStatus({strategies: 0, backtested: 0})).toBe('reading');
});

it('with no filter keeps everyone and counts every option', () => {
  const {ids, counts} = facetAuthors(facts, {});
  expect(ids.size).toBe(5);
  expect(counts.author_status).toEqual({backtested: 2, rules: 1, reading: 2});
  expect(counts.author_type).toEqual({person: 3, collection: 2});
});

it('combines filters, and counts each facet with the other facets applied', () => {
  const {ids, counts} = facetAuthors(facts, {author_status: 'reading', author_type: 'person'});
  expect([...ids]).toEqual(['tharp']);
  // Status counts honour Type = person; Type counts honour Status = reading.
  expect(counts.author_status).toEqual({backtested: 1, rules: 1, reading: 1});
  expect(counts.author_type).toEqual({person: 1, collection: 1});
  // A combination nobody has gives an empty list, never a fallback.
  expect(facetAuthors(facts, {author_status: 'rules', author_type: 'collection'}).ids.size).toBe(0);
});

it('the counts of one facet add up to the filtered total of the others', () => {
  const state = {author_type: 'collection'};
  const {ids, counts} = facetAuthors(facts, state);
  expect(Object.values(counts.author_status).reduce((a, b) => a + b, 0)).toBe(ids.size);
});

it('chips name each narrowing filter, ignore defaults, and Clear all removes all of them', () => {
  const state = {author_status: 'backtested', author_type: 'collection', book_category: 'money-management', library_scope: 'pending', library_q: 'gap', library_sort: 'name', library_market: 'us'};
  expect(activeFilters(state, 'authors').map(c => c.label)).toEqual(['Status: Backtested', 'Type: Collection', 'Topic: Money management', 'Library: Relevance review pending', '“gap”']);
  expect(activeFilters({library_scope: 'trading'}, 'authors')).toEqual([]);
  expect(activeFilters({...state, ...CLEAR_ALL}, 'authors')).toEqual([]);
  // Sort, market and view are choices, not filters: Clear all leaves them alone.
  expect(Object.keys(CLEAR_ALL)).not.toContain('library_sort');
  expect(Object.keys(CLEAR_ALL)).not.toContain('library_market');
  expect(activeFilters({book_progress: 'in_progress', author_status: 'rules'}, 'books').map(c => c.label)).toEqual(['Status: In progress']);
});
