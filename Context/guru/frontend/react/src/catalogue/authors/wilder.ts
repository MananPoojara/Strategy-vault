import type {AuthorFile} from '../author-files';

export default {
  order: 360,
  author: { id: 'wilder', name: 'J. Welles Wilder', tagline: 'New Concepts · Volatility, Directional Movement, Trend Balance Point, Reaction Trend', books: [{ id: 'wwnc', title: 'New Concepts in Technical Trading Systems', year: 1978, blurb: 'Wilder, 1978 (Spanish edition on the drive) · the book’s worked tables reproduce exactly; RSI has no exit in the book' }] },
  identities: ["J. Welles Wilder"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "Trend following",
      "Volatility"
    ],
    "summary": "J. Welles Wilder Jr. was an American mechanical engineer and real-estate developer best known for technical analysis. His 1978 book New Concepts in Technical Trading Systems introduced several indicators that became standard in charting software.",
    "known_for": [
      "Relative Strength Index (RSI)",
      "Average True Range (ATR)",
      "Directional Movement / ADX",
      "Parabolic SAR"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/J._Welles_Wilder_Jr.",
      "https://books.google.com/books/about/New_Concepts_in_Technical_Trading_System.html?id=WesJAQAAMAAJ"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "wwnc": {"type":"book","people":["J. Welles Wilder Jr."]},
  },
} satisfies AuthorFile;
