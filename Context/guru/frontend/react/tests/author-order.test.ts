import { expect, it } from "vitest";
import { orderAuthors } from "../src/services/author-order";

it("ranks real available backtests ahead of a large untested library", () => {
  const authors = [{ id: "reading", name: "Aaron" }, { id: "tested", name: "Zoe" }];
  const result = orderAuthors(authors, {
    counts: { reading: 500, tested: 1 },
    availability: { reading: { availableFamilies: 0 }, tested: { availableFamilies: 2 } },
  });
  expect(result.map((author) => author.id)).toEqual(["tested", "reading"]);
});

it("uses catalogue order for availability ties and stable input order without it", () => {
  const authors = [{ id: "a", name: "Aaron" }, { id: "z", name: "Zoe" }];
  expect(orderAuthors(authors, { sort: "available", positions: { a: 4, z: 0 } }).map((a) => a.id)).toEqual(["z", "a"]);
  expect(orderAuthors(authors).map((a) => a.id)).toEqual(["a", "z"]);
});

it("preserves explicit alphabetical, reverse and library-entry preferences", () => {
  const authors = [{ id: "z", name: "Zoe" }, { id: "a", name: "Aaron" }];
  const availability = { z: { availableFamilies: 100 }, a: { availableFamilies: 0 } };
  expect(orderAuthors(authors, { sort: "name", availability }).map((a) => a.id)).toEqual(["a", "z"]);
  expect(orderAuthors(authors, { sort: "name-desc", availability }).map((a) => a.id)).toEqual(["z", "a"]);
  expect(orderAuthors(authors, { sort: "books", availability, counts: { z: 1, a: 20 } }).map((a) => a.id)).toEqual(["a", "z"]);
});

it("rejects invalid measured counts and does not mutate frozen inputs", () => {
  const authors = Object.freeze([Object.freeze({ id: "invalid", name: "Invalid" }), Object.freeze({ id: "real", name: "Real" })]);
  const availability = Object.freeze({ invalid: Object.freeze({ availableFamilies: Infinity }), real: Object.freeze({ availableFamilies: 1 }) });
  const result = orderAuthors(authors, { availability });
  expect(result.map((a) => a.id)).toEqual(["real", "invalid"]);
  expect(authors.map((a) => a.id)).toEqual(["invalid", "real"]);
  expect(result[0]).toBe(authors[1]);
  expect(result).not.toBe(authors);
});
