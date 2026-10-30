import {expect, it} from 'vitest';
import {routeData} from '../src/services/route-data';

const known = {isCustom: (path: string) => path === '/research', isUsOnly: (id: string) => id === 'pristine one', isGuru: (id: string) => id === 'connors'};
const needs = (hash: string) => routeData(hash, known);

it('home, author and reading pages load the library; India pages never load US tickers', () => {
  for (const hash of ['', '#/', '#/guru/connors', '#/guru/connors?book=stc9', '#/source/abc', '#/anything-else'])
    expect(needs(hash)).toEqual({library: true, us: false});
});
it('reports, the ranked list and saved load neither', () => {
  for (const hash of ['#/strategies', '#/strategies?tk=NIFTY%2050&sort=cagr', '#/s/x', '#/o/x', '#/saved', '#/strategies?guru=connors'])
    expect(needs(hash)).toEqual({library: false, us: false});
});
it('a custom page loads nothing', () => {
  expect(needs('#/research')).toEqual({library: false, us: false});
  expect(needs('#/research?region=us')).toEqual({library: false, us: false});
});
it('US views load US tickers, on any page, and a US-only method does so without region in its link', () => {
  expect(needs('#/?region=us')).toEqual({library: true, us: true});
  expect(needs('#/s/x?region=us')).toEqual({library: false, us: true});
  expect(needs('#/strategies?region=all&tk=ALL')).toEqual({library: false, us: true});
  expect(needs('#/s/pristine%20one')).toEqual({library: false, us: true});
  expect(needs('#/s/%E0%A4%A')).toEqual({library: false, us: false});
});
it('a filter on an author only the library knows loads the library', () => {
  expect(needs('#/strategies?guru=source_group_7')).toEqual({library: true, us: false});
});
