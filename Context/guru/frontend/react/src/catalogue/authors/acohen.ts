import type {AuthorFile} from '../author-files';

export default {
  order: 840,
  author: { id: "acohen", name: "A. W. Cohen", tagline: "Chartcraft founder · the three-point reversal point & figure signal-to-signal rule runs here (US instruments: dollar box sizes)", books: [
    { id: "awcpf", sourceIds: ["title-fcfca896c4fe5bfb"], title: "How to Use the Three-Point Reversal Method of Point & Figure Stock Market Trading", year: 1984, blurb: "Cohen · buy on an X above the last X column, sell on an O below the last O column; trend lines and counts held" }] },
  identities: ["A. W. Cohen"],
  profile: {
    "kind": "person",
    "mastery": [
      "Point & figure charting"
    ],
    "summary": "A. W. Cohen founded Chartcraft and wrote the standard guide to three-point reversal point & figure charts of stocks, with signal formations, trend lines and price counts. This profile rests on the book itself.",
    "known_for": [
      "How to Use the Three-Point Reversal Method of Point & Figure Stock Market Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "awcpf": {"type":"book","people":["A. W. Cohen"]},
  },
} satisfies AuthorFile;
