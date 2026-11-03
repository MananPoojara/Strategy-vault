import type {AuthorFile} from '../author-files';

export default {
  order: 900,
  author: { id: "mcdonald", name: "Michael McDonald", tagline: "Author of Predict Market Swings With Technical Analysis · the moving-average timing and Bollinger trading-range rules of Ch.8 run here", books: [
    { id: "mcdpms", sourceIds: ["title-33042c847823ca05"], title: "Predict Market Swings With Technical Analysis", year: 2002, blurb: "McDonald · in above 98.5% of the 130-day average, out below (next-day close); 2-sd Bollinger sell/buy overlay of the 1966-1982 study" }] },
  identities: ["Michael McDonald"],
  profile: {
    "kind": "person",
    "mastery": [
      "Moving-average timing",
      "Market cycles",
      "Trading ranges"
    ],
    "summary": "Michael McDonald wrote Predict Market Swings With Technical Analysis (2002): a fair-value and feedback-loop model of the stock market and moving-average timing studies on the S&P 500 (1929-1998 and 1966-1982). This profile rests on the book itself.",
    "known_for": [
      "Predict Market Swings With Technical Analysis"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mcdpms": {"type":"book","people":["Michael McDonald"]},
  },
} satisfies AuthorFile;
