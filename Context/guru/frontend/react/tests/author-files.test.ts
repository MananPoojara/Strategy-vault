import {beforeAll, describe, expect, it} from 'vitest';
import {AUTHOR_FILES} from '../src/catalogue/author-files';
import {LIB} from '../src/compat/catalogue.js';
import {authorIdentities} from '../src/services/author-identities';
import {sourceReview} from '../src/services/source-review-data';

// Catalogue order of the 158 authors when the catalogue was split into one file per author (31 Oct 2026).
// Never edit this list: new authors go anywhere by their own `order`, but these keep their relative order.
const ORDER_AT_SPLIT = ['connors', 'chande', 'radge', 'covel', 'tharp', 'lwilliams', 'crabel', 'kaufman', 'harris', 'cooper', 'stridsman', 'jaekle', 'appel', 'pruitt', 'lebeau', 'stad', 'katz', 'bernstein', 'ehlers', 'demark', 'babcock', 'pardo', 'hirsch', 'krutsinger', 'dunnigan', 'ruggiero', 'momentum', 'bwilliams', 'landry', 'boucher', 'katsanos', 'weissman', 'bandy', 'meyers', 'clayburg', 'wilder', 'bollinger', 'elder', 'primo', 'weinstein', 'guppy', 'haggerty', 'quant', 'collins', 'kestner', 'seasonal', 'freeburg', 'gettess', 'lukac', 'timers', 'evidence', 'blau', 'sperandeo', 'berg', 'mtiming', 'pring', 'gregoriou', 'crisp', 'fabozzi', 'fontanills', 'thomsett', 'fischer', 'patel', 'vanvliet', 'kroll', 'jobman', 'langham', 'maclean', 'nison', 'kennedy', 'dittmann', 'duffy', 'wirrick', 'jcarter', 'borowski', 'merriman', 'csmith', 'lien', 'schlossberg', 'ponsi', 'downs', 'goodwin', 'cfwright', 'acohen', 'sadekar', 'gujral', 'sabatini', 'krausz', 'sheimo', 'mcdonald', 'melzl', 'graemesmith', 'sands', 'hyerczyk', 'linton', 'barrie', 'lcrane', 'burke', 'lakhi', 'schutzman', 'meissner', 'jjburns', 'lstephens', 'fpjackson', 'thayes', 'lowry', 'wooding', 'jennings', 'skyba', 'csatchwell', 'dbowden', 'dgagne', 'dpenn', 'ribakov', 'dorfman', 'deaton', 'bmillard', 'rrotella', 'tk1code', 'rlarsen', 'jlcava', 'adaptrade', 'nasdaqprofits', 'forman', 'kbrown', 'arms', 'kacher', 'bortucene', 'angell', 'augen', 'wyckoff', 'abell', 'etzkorn', 'carney', 'bickford', 'mdarcher', 'gbayer', 'person', 'bigalow', 'murphy', 'pesavento', 'sysmisc', 'swingstk', 'vendors', 'scart', 'schwager', 'swing12', 'magazines', 't2wgrp', 'v6misc', 'vcode', 'ross', 'raschke', 'minervini', 'oneil', 'darvas', 'livermore', 'pristine'];

describe('one file per author (src/catalogue/authors/<id>.ts)', () => {
  beforeAll(async () => {
    (globalThis as any).window ??= globalThis;
    // @ts-expect-error classic script that sets window.JoeRossEvidence (Ross's books)
    await import('../../joe-ross-evidence.js');
  });
  const files = import.meta.glob('../src/catalogue/authors/*.ts', {eager: true, import: 'default'}) as Record<string, (typeof AUTHOR_FILES)[number]>;
  it('names each file after its author id and gathers every file', () => {
    for (const [path, f] of Object.entries(files)) expect(path.endsWith('/' + f.author.id + '.ts'), path).toBe(true);
    expect(AUTHOR_FILES.length).toBe(Object.keys(files).length);
    expect(LIB.map((a: {id: string}) => a.id)).toEqual(AUTHOR_FILES.map(f => f.author.id));
  });
  it('keeps the catalogue order the authors had at the split', () => {
    const ids = LIB.map((a: {id: string}) => a.id);
    expect(ids.filter((id: string) => ORDER_AT_SPLIT.includes(id))).toEqual(ORDER_AT_SPLIT);
    for (const f of AUTHOR_FILES) expect(Number.isFinite(f.order), f.author.id).toBe(true);
  });
  it('lets each file hold only its own rows', () => {
    const books = AUTHOR_FILES.flatMap(f => f.author.books.map(b => b.id));
    expect(books.filter((b, i) => books.indexOf(b) !== i)).toEqual([]);
    for (const f of AUTHOR_FILES) {
      const own = new Set(f.author.books.map(b => b.id));
      expect(Object.keys(f.sourceReview ?? {}).filter(b => !own.has(b)), f.author.id).toEqual([]);
    }
    expect(Object.keys(sourceReview).length).toBe(AUTHOR_FILES.reduce((n, f) => n + Object.keys(f.sourceReview ?? {}).length, 0));
    expect(Object.keys(authorIdentities).every(id => AUTHOR_FILES.some(f => f.author.id === id))).toBe(true);
  });
});
