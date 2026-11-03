import type {AuthorFile} from '../author-files';

export default {
  order: 1180,
  author: { id: "rrotella", name: "Robert P. Rotella", tagline: "Futures trader and author · two Chapter 13 pattern rules run here", books: [
    { id: "rpets", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Robert P.Rotella - The Elements of Successful Trading/Rotella, R.P. -  The Elements of Successful Trading.PDF"], title: "The Elements of Successful Trading", year: 1992, blurb: "Rotella · fade a 1% opening gap from the open to that day's close; three higher (lower) closes in a row bought (sold) on the close and closed out at the close of the fifth day" }] },
  identities: ["Robert P. Rotella"],
  profile: {
    "kind": "person",
    "mastery": [
      "Futures trading",
      "Trading system testing"
    ],
    "summary": "Robert P. Rotella wrote The Elements of Successful Trading (New York Institute of Finance, 1992), a broad text on futures trading methods, pattern testing, money management and trader psychology. This profile rests on the book itself.",
    "known_for": [
      "The Elements of Successful Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rpets": {"type":"book","people":["Robert P. Rotella"]},
  },
} satisfies AuthorFile;
