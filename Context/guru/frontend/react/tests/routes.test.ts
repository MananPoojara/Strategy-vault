import { it, expect } from "vitest";
import { resolveRoute } from "../src/app/routes";
it("preserves old share links, queries and unknown-page errors", () => {
  expect(resolveRoute("#/").name).toBe("home");
  expect(resolveRoute("#/saved").name).toBe("saved");
  expect(resolveRoute("#/s/a%20b?tk=SPY").id).toBe("a b");
  expect(resolveRoute("#/o/r3").name).toBe("optimization");
  expect(resolveRoute("#/guru/tharp?book=tharp").name).toBe("author");
  expect(resolveRoute("#/source/source-a").name).toBe("reading");
  expect(resolveRoute("#/strategies").name).toBe("strategies");
  expect(resolveRoute("#/unknown").name).toBe("not-found");
  expect(resolveRoute("#/s/%ZZ").name).toBe("not-found");
});
