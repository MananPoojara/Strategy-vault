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
    const authors = list(LIB), byId = new Map(authors.map(author => [author.id, author]));
    const coverageAuthors = BOOK_COVERAGE?.authors || {};
    const audits = Array.isArray(coverageAuthors) ? coverageAuthors.map(author => [author.id, author]) : Object.entries(coverageAuthors);
    const auditById = new Map(audits.map(([id, author]) => [author.id || id, author]));
    const result = [], catalogueById = new Map(), sourceTargets = new Map();
    const associate = (row, authorId, authorName) => {
      row.authorIds = uniqueStrings([...row.authorIds, authorId]);
      row.authorNames = uniqueStrings([...row.authorNames, authorName]);
    };
    const linkSource = (sourceId, row) => {
      if (typeof sourceId !== 'string' || !sourceId) return;
      if (!sourceTargets.has(sourceId)) sourceTargets.set(sourceId, new Set());
      sourceTargets.get(sourceId).add(row);
    };
    for (const author of authors) for (const book of list(author.books)) {
      const row = { id: book.id, authorId: author.id, authorName: author.name || '', authorIds: [author.id], authorNames: [author.name || ''], title: book.title || '', year: validYear(book.year),
        ...assignedCategories(categoryLookup, book, author, 'catalogue'), kind: 'catalogue', book, author,
        aliases: aliasesFor(author, auditById.get(author.id), book), sourceIds: [], sources: [] };
      result.push(row);
      if (!catalogueById.has(book.id)) catalogueById.set(book.id, []);
      catalogueById.get(book.id).push(row);
      for (const sourceId of list(book.sourceIds)) linkSource(sourceId, row);
    }
    // Metadata title matches are discovery candidates, not proof of book/copy identity.
    // Only reviewed source IDs or the explicit SHA-verified association can merge a source into a book.
    for (const [, audit] of audits) for (const source of list(audit.source_titles)) {
      const candidates = [...list(source.catalogue_candidates), ...list(source.catalog_candidates)];
      for (const candidate of candidates) {
        if (candidate?.match !== 'curated_reading_original_sha256_identity_verified_not_strategy_certification') continue;
        for (const row of catalogueById.get(candidate.id) || []) linkSource(source.id, row);
      }
    }
    const sourcesById = new Map();
    for (const [key, audit] of audits) for (const source of list(audit.source_titles)) {
      if (typeof source.id !== 'string' || !source.id) continue;
      const authorId = audit.id || key, author = byId.get(authorId) || audit;
      const authorName = author.name || source.author_metadata || '';
      let row = sourcesById.get(source.id);
      if (!row) {
        row = { id: source.id, authorId, authorName, authorIds: [], authorNames: [], title: source.title || '', year: sourceYear(source),
          ...assignedCategories(categoryLookup, source, author, 'source'), kind: 'source', source, author,
          aliases: [], sourceIds: [source.id], sources: [] };
        sourcesById.set(source.id, row);
      }
      associate(row, authorId, authorName);
      row.sources.push(source);
      row.aliases = uniqueStrings([...row.aliases, ...aliasesFor(author, audit, source), source.title, authorName,
        ...list(source.files).map(file => file.filename)]);
    }
    for (const row of sourcesById.values()) {
      const targets = sourceTargets.get(row.id);
      if (!targets?.size) { result.push(row); continue; }
      for (const target of targets) {
        for (const authorId of row.authorIds) associate(target, authorId, '');
        for (const authorName of row.authorNames) associate(target, '', authorName);
        target.aliases = uniqueStrings([...target.aliases, ...row.aliases]);
        target.sourceIds = uniqueStrings([...target.sourceIds, row.id]);
        target.sources.push(...row.sources);
      }
    }
    return result;
  }
  const categoryRequested = category => category != null && normalize(category) !== '' && normalize(category) !== 'all';
  const searchable = record => normalize([
    record.title, record.authorName, ...list(record.authorNames), ...list(record.aliases), record.book?.blurb, record.book?.summary, record.source?.summary,
    ...list(record.source?.files).map(file => file.filename), record.source?.author_metadata,
    ...list(record.sources).flatMap(source => [source.title, source.summary, source.author_metadata, ...list(source.files).map(file => file.filename)])
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
    const ids = new Set(search(input, { query, category }).flatMap(record => list(record.authorIds).length ? record.authorIds : [record.authorId]));
    return groups.filter(author => ids.has(author.id) || !useCategory && (!term || normalize([author.name, ...list(author.aliases)].join(' ')).includes(term)));
  }
  return Object.freeze({ records, search, matchingAuthors });
});
