import type {AuthorFile} from '../author-files';

export default {
  order: 350,
  author: { id: 'clayburg', name: 'John Clayburg', tagline: 'Self-adaptive systems · nothing testable on daily data', books: [{ id: 'clfs', title: 'Four Steps to Trading Success', year: 2001, blurb: 'Judged by eye on 1–2 minute charts' }, { id: 'clomw', title: 'Parallel Functions in Self-Adaptive Systems', year: 2000, blurb: 'Needs intraday bars' }] },
  identities: ["John Clayburg"],
  profile: {
    "kind": "person",
    "mastery": [
      "Day trading",
      "System design & testing"
    ],
    "summary": "Dr. John F. Clayburg, a veterinarian by training, has developed trading systems for more than 20 years. He is known for the Cyclone S&P day-trading system, which Futures Truth often ranked near the top, and for Parallel User Function Technology, a self-adaptive system platform.",
    "known_for": [
      "Four Steps to Trading Success",
      "Cyclone S&P day-trading system",
      "Parallel (self-adaptive) functions"
    ],
    "sources": [
      "https://www.clayburg.com/four_steps_to_trading_success.htm"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "clfs": {"type":"book","people":["John Clayburg"]},
    "clomw": {"type":"article","people":["John Clayburg"]},
  },
} satisfies AuthorFile;
