import type {AuthorFile} from '../author-files';

export default {
  order: 910,
  author: { id: "melzl", name: "Mark Melzl", tagline: "Compiler of The TradeStation Cookbook · six printed daily systems run here; systems with missing pages or functions held", books: [
    { id: "tscb2", sourceIds: ["title-e54ec3553f6be961"], title: "The TradeStation Cookbook II", year: 1997, blurb: "Melzl (compiler) · James Bond %K(100), Daily Smash YX, BB AVG, Consecutive Closes, CCI 200, Average Volume (scanned code listing)" }] },
  identities: ["Mark Melzl"],
  profile: {
    "kind": "person",
    "mastery": [
      "TradeStation EasyLanguage"
    ],
    "summary": "Mark Melzl compiled The TradeStation Cookbook I and II (GraphFax, 1997): a TradeStation 4.0 how-to manual and a listing of indicator and system code. This profile rests on the books themselves.",
    "known_for": [
      "The TradeStation Cookbook"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "tscb2": {"type":"book","people":["Mark Melzl"]},
  },
} satisfies AuthorFile;
