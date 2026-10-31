import type {AuthorFile} from '../author-files';

export default {
  order: 210,
  author: { id: 'babcock', name: 'Bruce Babcock', tagline: 'Reality Based Trading · tested S&P systems and the OOPS day trade', books: [{ id: 'bab80', title: 'The 80% Solution: S&P Systems', year: 1997, blurb: 'Babcock, 1997 · day-trade and short-term momentum breakouts, Nov–Apr calendar system with and without stops' }, { id: 'baboops', title: 'The Best Mechanical Day Trading System I Know', year: 1997, blurb: 'Babcock, 1997 · OOPS and STOOPS as tested; STOOPS as traded needs intraday bars' }] },
  identities: ["Bruce Babcock"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Futures",
      "Day trading"
    ],
    "summary": "Bruce Babcock Jr. (died 1998) was a lawyer and former federal prosecutor who became a commodity trader in 1979. He published the Commodity Traders Consumer Report, which rated advisory services, trading systems and software. His books include mechanical S&P futures day-trading systems.",
    "known_for": [
      "Commodity Traders Consumer Report",
      "The 80% Solution: S&P Systems"
    ],
    "sources": [
      "https://yale1965.org/bruce-babcock-jr/",
      "https://store.traders.com/-v04-c05-ctc-pdf.html"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "bab80": {"type":"other","people":["Bruce Babcock"]},
    "baboops": {"type":"other","people":["Bruce Babcock"]},
  },
} satisfies AuthorFile;
