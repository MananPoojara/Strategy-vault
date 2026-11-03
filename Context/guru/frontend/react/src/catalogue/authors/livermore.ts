import type {AuthorFile} from '../author-files';

export default {
  order: 1570,
  author: { id: 'livermore', name: 'Jesse Livermore', books: [{ id: 'howtotrade', title: 'How to Trade in Stocks', year: 1940 }] },
  identities: ["Edwin LeFevre","Jesse Livermore"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Breakout",
      "Trading psychology"
    ],
    "summary": "Jesse Livermore (1877-1940) was an American speculator and the model for Reminiscences of a Stock Operator. He traded with what is now called technical analysis. How to Trade in Stocks (1940) sets out his pivotal-point method for entering on confirmed trend moves and stresses emotional control.",
    "known_for": [
      "How to Trade in Stocks",
      "Pivotal points",
      "Reminiscences of a Stock Operator (subject)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Jesse_Livermore"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "howtotrade": {"type":"book","people":["Jesse Livermore"]},
  },
} satisfies AuthorFile;
