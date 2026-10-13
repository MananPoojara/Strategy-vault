import { it, expect } from "vitest";
import { libraryScope, inLibraryScope } from "../src/services/library-scope";
const curated = new Set(["tharp"]);
it("keeps plotting and general programming guides outside the trading library", () => {
  expect(
    libraryScope(
      {
        kind: "source",
        title: "Matplotlib User Guide",
        authorId: "source-credit",
      },
      curated,
    ),
  ).toBe("technical");
  expect(
    libraryScope(
      { kind: "source", title: "Learning Python", authorId: "source-credit" },
      curated,
    ),
  ).toBe("technical");
  expect(
    libraryScope(
      {
        kind: "source",
        title: "Document123",
        authorId: "source-credit",
        source: { files: [{ filename: "Mastering-Matplotlib.pdf" }] },
      },
      curated,
    ),
  ).toBe("technical");
});
it("recognizes financial context without claiming rules or performance", () => {
  expect(
    libraryScope(
      {
        kind: "source",
        title: "Python for Algorithmic Trading",
        authorId: "new-credit",
      },
      curated,
    ),
  ).toBe("trading");
  expect(
    libraryScope(
      {
        kind: "source",
        title: "Position Sizing and Money Management",
        authorId: "new-credit",
      },
      curated,
    ),
  ).toBe("trading");
  expect(
    libraryScope(
      {
        kind: "source",
        title: "Stop Worrying About Yourself",
        authorId: "tharp",
      },
      curated,
    ),
  ).toBe("trading");
  expect(
    libraryScope(
      { kind: "source", title: "Unknown title", authorId: "unreviewed" },
      curated,
    ),
  ).toBe("pending");
  expect(
    libraryScope(
      { kind: "catalogue", title: "Known curated book", authorId: "tharp" },
      curated,
    ),
  ).toBe("trading");
});
it("uses reviewed identity over filenames, retains the full inventory explicitly", () => {
  expect(
    libraryScope(
      {
        kind: "source",
        title: "Matplotlib.pdf",
        authorId: "new",
        source: {
          source_identity: { title: "Technical Analysis for Traders" },
        },
      },
      curated,
    ),
  ).toBe("trading");
  expect(inLibraryScope("technical", "trading")).toBe(false);
  expect(inLibraryScope("technical", "all")).toBe(true);
  expect(inLibraryScope("pending", "pending")).toBe(true);
});
