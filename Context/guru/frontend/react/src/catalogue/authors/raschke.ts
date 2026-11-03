import type {AuthorFile} from '../author-files';

export default {
  order: 1530,
  author: { id: 'raschke', name: 'Linda Raschke', tagline: 'Short-term swing patterns · Street Smarts is listed under Larry Connors', books: [{ id: 'lbrtt', title: 'Trading Techniques 2008 (workshop manual)', year: 2008, blurb: 'LBRGroup, 2008 · the two rules stated fully enough to test' }, { id: 'lbrvb', title: "Volatility Breakout Systems (article)", year: null, blurb: "Raschke · a template without numbers, nothing testable" }] },
  identities: ["Linda Raschke"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term swing",
      "Pattern recognition",
      "Futures"
    ],
    "summary": "Linda Bradford Raschke is a US futures trader who began as an options market maker in 1981, became a CTA in 1992 and ran hedge funds until 2015. She was profiled in The New Market Wizards. She is known for short-term technical setups and co-wrote Street Smarts with Laurence Connors.",
    "known_for": [
      "Street Smarts (with Connors)",
      "Trading Sardines",
      "Short-term setups and volatility breakouts"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Linda_Bradford_Raschke",
      "https://lindaraschke.net/bio-linda/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "lbrtt": {"type":"course_manual","people":["Linda Bradford Raschke"]},
    "lbrvb": {"type":"article","people":["Linda Bradford Raschke"]},
  },
} satisfies AuthorFile;
