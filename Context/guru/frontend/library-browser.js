/* Pure library discovery. Category labels and source identity never certify trading rules or performance. */
(function (root, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.VaultLibraryBrowser = api;
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';
  const list = value => Array.isArray(value) ? value : [];
  const normalize = value => String(value ?? '').normalize('NFKD').replace(/\p{M}/gu, '').toLocaleLowerCase('en').replace(/\s+/g, ' ').trim();
  const categoryMap = {
    'rule-based': 'rule-based', 'rule based': 'rule-based', rule_based: 'rule-based', rules: 'rule-based',
    psychology: 'psychology', psychological: 'psychology',
    'money-management': 'money-management', 'money management': 'money-management', money_management: 'money-management', money: 'money-management'
  };
  const categoryId = value => categoryMap[normalize(value)] || null;
  const uniqueStrings = values => [...new Set(values.filter(v => typeof v === 'string' && v.trim()))];
  const validYear = value => {
    if (!(typeof value === 'number' || typeof value === 'string' && /^\d{4}$/.test(value.trim()))) return null;
    const year = Number(value);
    return Number.isInteger(year) && year >= 1000 && year <= 2999 ? year : null;
  };
  const sourceYear = source => {
    const explicit = validYear(source.year);
    if (explicit !== null) return explicit;
    const years = [...new Set(list(source.years_reported).map(validYear).filter(y => y !== null))];
    return years.length === 1 ? years[0] : null;
  };
  function assignedCategories(lookup, item, author, kind) {
    const value = typeof lookup === 'function' ? lookup(item.id, item, author, kind) : lookup && Object.prototype.hasOwnProperty.call(lookup, item.id) ? lookup[item.id] : undefined;
    const declared = value && typeof value === 'object' && !Array.isArray(value) ? value.categories : value;
    const categories = [...new Set((Array.isArray(declared) ? declared : [declared]).map(categoryId).filter(Boolean))];
    return { categories, categoryCertainty: categories.length ? String(value?.categoryCertainty || value?.certainty || 'editorial') : 'unassigned' };
  }
  function aliasesFor(author, audit, item) {
    return uniqueStrings([
      ...list(author?.aliases), ...list(author?.author_alias_keys), ...list(author?.source_author_spellings),
      ...list(audit?.aliases), ...list(audit?.author_alias_keys), ...list(audit?.source_author_spellings),
      ...list(item?.aliases), ...(typeof item?.author_metadata === 'string' ? [item.author_metadata] : [])
    ]);
  }
  function records(LIB, BOOK_COVERAGE, categoryLookup) {
    const authors = list(LIB), knownBooks = new Set(authors.flatMap(author => list(author.books).map(book => book.id))), byId = new Map(authors.map(author => [author.id, author]));
    const coverageAuthors = BOOK_COVERAGE?.authors || {};
    const audits = Array.isArray(coverageAuthors) ? coverageAuthors.map(author => [author.id, author]) : Object.entries(coverageAuthors);
    const auditById = new Map(audits.map(([id, author]) => [author.id || id, author]));
    const result = [];
    for (const author of authors) for (const book of list(author.books)) {
      result.push({ id: book.id, authorId: author.id, authorName: author.name || '', title: book.title || '', year: validYear(book.year),
        ...assignedCategories(categoryLookup, book, author, 'catalogue'), kind: 'catalogue', book, author,
        aliases: aliasesFor(author, auditById.get(author.id), book) });
    }
    // A known catalogue relationship anywhere in the inventory excludes the duplicate source row everywhere.
    const suppressed = new Set();
    for (const [, audit] of audits) for (const source of list(audit.source_titles)) {
      const candidates = [...list(source.catalogue_candidates), ...list(source.catalog_candidates)];
      if (candidates.some(candidate => knownBooks.has(typeof candidate === 'string' ? candidate : candidate?.id))) suppressed.add(source.id);
    }
    const seen = new Set();
    for (const [key, audit] of audits) for (const source of list(audit.source_titles)) {
      if (typeof source.id !== 'string' || !source.id || seen.has(source.id) || suppressed.has(source.id)) continue;
      seen.add(source.id);
      const authorId = audit.id || key, author = byId.get(authorId) || audit;
      result.push({ id: source.id, authorId, authorName: author.name || source.author_metadata || '', title: source.title || '', year: sourceYear(source),
        ...assignedCategories(categoryLookup, source, author, 'source'), kind: 'source', source, author,
        aliases: aliasesFor(author, audit, source) });
    }
    return result;
  }
  const categoryRequested = category => category != null && normalize(category) !== '' && normalize(category) !== 'all';
  const searchable = record => normalize([
    record.title, record.authorName, ...list(record.aliases), record.book?.blurb, record.book?.summary, record.source?.summary,
    ...list(record.source?.files).map(file => file.filename), record.source?.author_metadata
  ].filter(value => value != null).join(' '));
  const compare = new Intl.Collator('en', { sensitivity: 'base', numeric: true }).compare;
  function search(input, { query = '', category = '', sort = 'original' } = {}) {
    const term = normalize(query), useCategory = categoryRequested(category), wanted = categoryId(category);
    const rows = list(input).filter(record => (!term || searchable(record).includes(term)) && (!useCategory || wanted && list(record.categories).some(value => categoryId(value) === wanted)));
    const mode = normalize(sort);
    const titleAsc = ['title', 'title-asc', 'az', 'a-z', 'name', 'name-asc'].includes(mode);
    const titleDesc = ['title-desc', 'za', 'z-a', 'name-desc'].includes(mode);
    const authorSort = ['author', 'author-asc'].includes(mode);
    const newest = ['newest', 'year-desc', 'recent'].includes(mode);
    if (!titleAsc && !titleDesc && !authorSort && !newest) return rows;
    return rows.map((record, index) => ({ record, index })).sort((a, b) => {
      let difference = 0;
      if (newest) {
        const ay = validYear(a.record.year), by = validYear(b.record.year);
        difference = ay === null ? by === null ? 0 : 1 : by === null ? -1 : by - ay;
      } else difference = compare(String(authorSort ? a.record.authorName || '' : a.record.title || ''), String(authorSort ? b.record.authorName || '' : b.record.title || '')) * (titleDesc ? -1 : 1);
      return difference || a.index - b.index;
    }).map(item => item.record);
  }
  function matchingAuthors(LIB, input, query = '', category = '') {
    const groups = list(LIB), term = normalize(query), useCategory = categoryRequested(category);
    const ids = new Set(search(input, { query, category }).map(record => record.authorId));
    return groups.filter(author => ids.has(author.id) || !useCategory && (!term || normalize([author.name, ...list(author.aliases)].join(' ')).includes(term)));
  }
  return Object.freeze({ records, search, matchingAuthors });
});
