export interface AuthorIdentity {
  id: string;
  name: string;
}

export interface AuthorOrderingOptions {
  sort?: string;
  counts?: Readonly<Record<string, number | undefined>>;
  availability?: Readonly<
    Record<string, { availableFamilies?: number } | undefined>
  >;
  positions?: Readonly<Record<string, number | undefined>>;
}

function measuredCount(value: number | undefined): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0
    ? value
    : 0;
}

function cataloguePosition(value: number | undefined): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0
    ? value
    : Infinity;
}

/** Order cards by the measured counts for the context their cards display.
 * The caller selects and labels that context. Registration, admission flags,
 * strategy counts and results on another instrument cannot imply availability.
 * Catalogue position resolves ties; missing/equal positions preserve input order.
 * A new array is returned, preserving the original author objects and inputs.
 */
export function orderAuthors<T extends AuthorIdentity>(
  authors: readonly T[],
  options: AuthorOrderingOptions = {},
): T[] {
  const records = authors.map((author, index) => ({
    author,
    index,
    available: measuredCount(
      options.availability?.[author.id]?.availableFamilies,
    ),
    entries: measuredCount(options.counts?.[author.id]),
    position: cataloguePosition(options.positions?.[author.id]),
  }));

  records.sort((left, right) => {
    let comparison: number;
    switch (options.sort) {
      case "name":
        comparison = left.author.name.localeCompare(right.author.name);
        break;
      case "name-desc":
        comparison = right.author.name.localeCompare(left.author.name);
        break;
      case "books":
        comparison = right.entries - left.entries;
        break;
      default:
        comparison = right.available - left.available;
    }
    if (comparison !== 0) return comparison;
    if (left.position !== right.position) return left.position - right.position;
    return left.index - right.index;
  });

  return records.map(({ author }) => author);
}
