import type {AuthorFile} from '../author-files';

export default {
  order: 1050,
  author: { id: "thayes", name: "Timothy Hayes", tagline: "Ned Davis Research strategist · the 30-week rate-of-change brackets run here", books: [
    { id: "thrdi", sourceIds: [], title: "The Research Driven Investor", year: 2001, blurb: "Hayes · buy when the 30-week rate of change rises back above 3% after dipping below, switch to cash when it falls back below 15% after rising above (Fig 4-9); the other figure indicators are held" },
    { id: "thcop", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Teresa Lo Traders Master Class Transcripts/TeresaLo and other docs/Coppock_Hayes.pdf"], title: "The Coppock Guide (S&C, 1993)", year: 1993, blurb: "Hayes \u00b7 monthly Coppock guide: buy when it turns up from below 1.3, sell after an 11-point drop from its top (DJIA 1957-92: 90% profitable)" }] },
  identities: ["Timothy Hayes"],
  profile: {
    "kind": "person",
    "mastery": [
      "Market indicators",
      "Indicator research"
    ],
    "summary": "Timothy W. Hayes, a strategist at Ned Davis Research, wrote The Research Driven Investor (2001), which explains how to build and test market indicators from price, breadth, sentiment and economic data, with many NDR chart examples. This profile rests on the book itself.",
    "known_for": [
      "The Research Driven Investor"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "thrdi": {"type":"book","people":["Timothy Hayes"]},
    "thcop": {"type":"article","people":["Timothy Hayes"]},
  },
} satisfies AuthorFile;
