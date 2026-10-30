/** Which big files a page reads. The library files (about 100 MB) belong to the home, author and reading pages;
 * the US ticker list (about 35 MB) only to US views. A strategy report or the ranked list needs neither. */
export function routeData(hash: string, known: {isCustom(path: string): boolean; isUsOnly(id: string): boolean; isGuru(id: string): boolean}) {
  const [path = '/', search = ''] = hash.replace(/^#/, '').split('?');
  if (known.isCustom(path)) return {library: false, us: false};
  const [page, id] = path.split('/').filter(Boolean), query = new URLSearchParams(search), region = query.get('region');
  const report = (page === 's' || page === 'o') && !!id;
  let usOnly = false;
  if (report) try {usOnly = known.isUsOnly(decodeURIComponent(id));} catch { /* A malformed link is not a US view. */ }
  const guru = query.get('guru');
  return {
    us: region === 'us' || region === 'all' || usOnly,
    library: !(report || page === 'saved' || page === 'strategies') || (page === 'strategies' && !!guru && !known.isGuru(guru)),
  };
}
