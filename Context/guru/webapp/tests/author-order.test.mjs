import test from "node:test";
import assert from "node:assert/strict";
import { orderAuthors } from "../../frontend/react/src/services/author-order.ts";

const ids = (authors) => authors.map((author) => author.id);

test("default order puts measured results above registrations without results", () => {
  const authors = [
    { id: "registered", name: "Aaron", strategies: 300 },
    { id: "measured", name: "Zoe", strategies: 1 },
  ];
  const availability = {
    registered: { live: 300, admitted: true, status: "ready" },
    measured: { availableFamilies: 1 },
  };
  assert.deepEqual(ids(orderAuthors(authors, { availability })), [
    "measured",
    "registered",
  ]);
});

test("available sort uses descending displayed measured family counts", () => {
  const authors = [
    { id: "one", name: "Aaron" },
    { id: "eight", name: "Zoe" },
    { id: "three", name: "Mike" },
  ];
  const availability = {
    one: { availableFamilies: 1 },
    eight: { availableFamilies: 8 },
    three: { availableFamilies: 3 },
  };
  assert.deepEqual(ids(orderAuthors(authors, { sort: "available", availability })), [
    "eight",
    "three",
    "one",
  ]);
});

test("availability ties follow catalogue position rather than author name", () => {
  const authors = [
    { id: "a", name: "Aaron" },
    { id: "z", name: "Zoe" },
    { id: "m", name: "Mike" },
  ];
  const availability = Object.fromEntries(authors.map(({ id }) => [id, { availableFamilies: 2 }]));
  assert.deepEqual(
    ids(orderAuthors(authors, { availability, positions: { a: 5, z: 0, m: 2 } })),
    ["z", "m", "a"],
  );
});

test("missing or identical catalogue positions retain stable input order", () => {
  const authors = [
    { id: "z", name: "Zoe" },
    { id: "a", name: "Aaron" },
    { id: "m", name: "Mike" },
  ];
  assert.deepEqual(ids(orderAuthors(authors)), ["z", "a", "m"]);
  assert.deepEqual(ids(orderAuthors(authors, { positions: { z: 0, a: 0, m: 0 } })), ["z", "a", "m"]);
});

test("explicit alphabetical directions ignore availability", () => {
  const authors = [{ id: "z", name: "Zoe" }, { id: "a", name: "Aaron" }];
  const availability = { z: { availableFamilies: 90 }, a: { availableFamilies: 0 } };
  assert.deepEqual(ids(orderAuthors(authors, { sort: "name", availability })), ["a", "z"]);
  assert.deepEqual(ids(orderAuthors(authors, { sort: "name-desc", availability })), ["z", "a"]);
});

test("explicit library-entry sort uses entry counts rather than backtest counts", () => {
  const authors = [{ id: "measured", name: "Aaron" }, { id: "reading", name: "Zoe" }];
  assert.deepEqual(ids(orderAuthors(authors, {
    sort: "books",
    counts: { measured: 1, reading: 20 },
    availability: { measured: { availableFamilies: 100 } },
  })), ["reading", "measured"]);
});

test("invalid measurement fields never invent available families", () => {
  const authors = [
    { id: "negative", name: "Negative" },
    { id: "infinite", name: "Infinite" },
    { id: "nan", name: "NaN" },
    { id: "fraction", name: "Fraction" },
    { id: "text", name: "Text" },
    { id: "real", name: "Real" },
  ];
  const availability = {
    negative: { availableFamilies: -2 },
    infinite: { availableFamilies: Infinity },
    nan: { availableFamilies: NaN },
    fraction: { availableFamilies: 1.5 },
    text: { availableFamilies: "99" },
    real: { availableFamilies: 1 },
  };
  assert.deepEqual(ids(orderAuthors(authors, { availability })), [
    "real", "negative", "infinite", "nan", "fraction", "text",
  ]);
});

test("only the provided context measurement affects order", () => {
  const authors = [{ id: "other-market", name: "Other" }, { id: "selected-market", name: "Selected" }];
  const availability = {
    "other-market": { availableFamilies: 0, globalFamilies: 500, instrument: "SPY" },
    "selected-market": { availableFamilies: 2, instrument: "NIFTY 50" },
  };
  assert.deepEqual(ids(orderAuthors(authors, { availability })), ["selected-market", "other-market"]);
});

test("sorting does not mutate authors, measurements, counts or positions", () => {
  const authors = Object.freeze([
    Object.freeze({ id: "a", name: "Aaron", note: "retained" }),
    Object.freeze({ id: "z", name: "Zoe", note: "retained too" }),
  ]);
  const availability = Object.freeze({ a: Object.freeze({ availableFamilies: 0 }), z: Object.freeze({ availableFamilies: 3 }) });
  const counts = Object.freeze({ a: 9, z: 1 });
  const positions = Object.freeze({ a: 0, z: 1 });
  const result = orderAuthors(authors, { availability, counts, positions });
  assert.deepEqual(ids(authors), ["a", "z"]);
  assert.deepEqual(ids(result), ["z", "a"]);
  assert.notEqual(result, authors);
  assert.equal(result[0], authors[1]);
  assert.equal(result[1].note, "retained");
});

test("an unknown saved sort falls back to measured availability, and empty input stays empty", () => {
  assert.deepEqual(orderAuthors([]), []);
  const authors = [{ id: "a", name: "Aaron" }, { id: "z", name: "Zoe" }];
  assert.deepEqual(ids(orderAuthors(authors, {
    sort: "removed-sort",
    availability: { z: { availableFamilies: 2 } },
  })), ["z", "a"]);
});
