import type {AuthorFile} from '../author-files';

export default {
  order: 750,
  author: { id: "borowski", name: "Robert Borowski", tagline: "Rapid Forex e-books · intraday wave surfing, scalping and simple/exponential average trend lines on currency charts (held)", books: [
    { id: "rbfsurf", sourceIds: ["title-7c06ca1d8e3dfdab", "title-bb98231bb40e960f"], title: "Forex Surfing", year: null, blurb: "Borowski · intraday wave and channel trades; held: lines by hand, pip sizes" },
    { id: "rbfsail", sourceIds: ["title-fcae7827351fd252"], title: "Forex Sailing", year: null, blurb: "Borowski · S.E.X. 5/20/60 lines; held: 'not meant to be used alone'" },
    { id: "rbfscalp", sourceIds: ["title-7cd7d14bd81aaa1f"], title: "Forex Scalping", year: null, blurb: "Borowski · 1-minute scalps; held: intraday forex, pip stops" }] },
  identities: ["Robert Borowski"],
  profile: {
    "kind": "person",
    "mastery": [
      "Forex trading"
    ],
    "summary": "Robert Borowski published the Rapid Forex e-books on short-term and trend trading in currencies. They describe wave, channel and moving-average techniques read on intraday and daily forex charts, with stops and targets in pips. This profile rests on the books themselves.",
    "known_for": [
      "Forex Surfing",
      "Forex Sailing",
      "Forex Scalping"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rbfsurf": {"type":"book","people":["Robert Borowski"]},
    "rbfsail": {"type":"book","people":["Robert Borowski"]},
    "rbfscalp": {"type":"book","people":["Robert Borowski"]},
  },
} satisfies AuthorFile;
