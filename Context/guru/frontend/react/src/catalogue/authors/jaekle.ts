import type {AuthorFile} from '../author-files';

export default {
  order: 120,
  author: { id: 'jaekle', name: 'Urban Jaekle & Emilio Tomasini', tagline: 'Building and testing a trading system end to end', books: [{ id: 'jt', title: 'Trading Systems (2nd ed.)', year: 2019, blurb: 'Jaekle & Tomasini, 2019 · LUXOR moving-average system, Bollinger systems, beginning-of-month · the 30-minute FX and portfolio versions need data' }] },
  identities: ["Emilio Tomasini","Urban Jaekle"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Futures"
    ],
    "summary": "Urban Jaekle and Emilio Tomasini co-wrote Trading Systems (Harriman House), a guide to building, testing and portfolio-optimising mechanical systems; the 2nd edition adds AmiBroker code. Jaekle has traded stocks since 1995 and futures since 2000 and runs a systematic stock advisory service.",
    "known_for": [
      "Trading Systems: A New Approach to System Development and Portfolio Optimisation"
    ],
    "sources": [
      "https://harriman.house/books/trading-systems-2nd-edition/",
      "https://legacy.harriman-house.com/press/full/4051/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "jt": {"type":"book","people":["Emilio Tomasini","Urban Jaekle"]},
  },
} satisfies AuthorFile;
