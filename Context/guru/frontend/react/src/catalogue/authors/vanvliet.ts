import type {AuthorFile} from '../author-files';

export default {
  order: 640,
  author: { id: "vanvliet", name: "Benjamin Van Vliet", tagline: "Automated-trading software and process engineering · C++/.NET and VB.NET example systems", books: [
    { id: "vvbats", sourceIds: ["title-7d5fab9b2ef45e33", "title-d9d7febf0c69c956"], title: "Building Automated Trading Systems", year: 2007, blurb: "Van Vliet · C++/.NET architecture; held: the tick MA and ES/NQ spread examples need futures ticks and quotes" },
    { id: "vvmfm", sourceIds: ["title-3801128c46565f3f", "title-04fe541148df6896"], title: "Modeling Financial Markets (with Hendry)", year: 2004, blurb: "Van Vliet & Hendry · VB.NET pricing and trading models; programming examples only" },
    { id: "vvqmm", sourceIds: ["title-b27c205cebe0fc83"], title: "Quality Money Management (with Kumiega)", year: 2008, blurb: "Kumiega & Van Vliet · process engineering; held: pairs and factor examples need a named pair, fundamentals and options" }] },
  identities: ["Benjamin Van Vliet"],
  profile: {
    "kind": "person",
    "mastery": [
      "Automated trading systems",
      "Trading-system development process"
    ],
    "summary": "Benjamin Van Vliet teaches and writes on building automated trading systems. His books in the library show how to program and engineer trading and pricing systems in C++/.NET and VB.NET; their example systems are illustrations (futures ticks, spreads, factor rankings) that he says would not make money as written, so no complete mechanical rule is given. This profile rests on the books themselves.",
    "known_for": [
      "Building Automated Trading Systems",
      "Quality Money Management"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "vvbats": {"type":"book","people":["Benjamin Van Vliet"]},
    "vvmfm": {"type":"book","people":["Benjamin Van Vliet"]},
    "vvqmm": {"type":"book","people":["Benjamin Van Vliet"]},
  },
} satisfies AuthorFile;
