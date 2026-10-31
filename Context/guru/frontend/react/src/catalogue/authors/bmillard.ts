import type {AuthorFile} from '../author-files';

export default {
  order: 1170,
  author: { id: "bmillard", name: "Brian J. Millard", tagline: "UK share analyst and author · the five-week turn with the 41-week trend estimate runs here", books: [
    { id: "bmwsm", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Brian J.Millard - Winning on the Stock Market/Brian J.Millard - Winning on the Stock Market.pdf"], title: "Winning on the Stock Market", year: 1998, blurb: "Millard · weekly: buy when the five-week average turns up on a rising close above the close three weeks ago while the 41-week price-analysis estimate is rising; out on a close 7.5% under the highest close since purchase or after 20 weeks" }] },
  identities: ["Brian J. Millard"],
  profile: {
    "kind": "person",
    "mastery": [
      "UK share investing",
      "Moving averages"
    ],
    "summary": "Brian J. Millard wrote Winning on the Stock Market (Wiley, 2nd edition 1998), a guide for private investors in UK shares built on stop-loss floors, the dominant trend of the 41-week average estimated ahead by price analysis, and five-week average buying signals. This profile rests on the book itself.",
    "known_for": [
      "Winning on the Stock Market"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "bmwsm": {"type":"book","people":["Brian J. Millard"]},
  },
} satisfies AuthorFile;
