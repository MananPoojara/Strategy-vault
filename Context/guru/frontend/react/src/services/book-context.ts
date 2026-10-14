/** Publication metadata never supplies an invented original study window. */
export function bookContext(
  id: string,
  windows: Record<string, readonly [string, string, number]>,
  books: Record<string, {year?: unknown}>,
): [string, string, number | null] {
  const explicit = windows[id];
  if (explicit) return [...explicit];
  const year = Number(books[id]?.year);
  return ['', '', Number.isInteger(year) && year > 0 && year <= 9999 ? year : null];
}
