import type {AuthorFile} from '../author-files';

export default {
  order: 20,
  author: { id: 'chande', name: 'Tushar Chande', books: [{ id: 'ch_bta', title: 'Beyond Technical Analysis', year: 1997, blurb: 'Supplied partial edition · 65sma-3cc and CB-PB OHLC market adaptations. Contract dollar-stop variants remain Data missing; original futures studies are not reproduced.' }, { id: 'chbta2', title: 'Beyond Technical Analysis (complete edition)', year: 1997, blurb: 'Chande, 1997 · the rest of the book: RAVI-filtered 65sma-3cc, S&P 10/11 crossovers, bottom-fishing, channel breakouts, ADX systems, long bomb · two-market and contract-stop systems need data' }, { id: 'chntt', title: 'The New Technical Trader (with Stanley Kroll)', year: 1994, blurb: 'Chande & Kroll, 1994 · CMO-driven VIDYA system and the thrust-oscillator buy signal; the other indicators come without a complete rule' }, { id: 'chpbs', title: "Developing & Implementing Pattern-Based Trading Systems (workshop)", year: 1996, blurb: "Chande, TAG 1996 · channel-breakout pullback with the 3-day-low trail; no initial stop by default" }] },
  identities: ["Tushar Chande"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "System design & testing",
      "Futures"
    ],
    "summary": "Tushar Chande, PhD, is an engineer turned commodity trading advisor who managed systematic futures money for about two decades. He created widely used indicators including Aroon, the Chande Momentum Oscillator, StochRSI and VIDYA, and wrote on trading-system development.",
    "known_for": [
      "Aroon",
      "Chande Momentum Oscillator",
      "StochRSI and VIDYA",
      "Beyond Technical Analysis"
    ],
    "sources": [
      "https://bethepuck.com/thinkers/tushar-chande/",
      "https://seekingalpha.com/author/tushar-chande",
      "https://traders.com/documentation/feedbk_docs/1997/10/abstracts_new/interview/interview9710.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "ch_bta": {"type":"book","sameWorkAs":"chbta2","people":["Tushar S. Chande"]},
    "chbta2": {"type":"book","people":["Tushar S. Chande"]},
    "chntt": {"type":"book","people":["Stanley Kroll","Tushar S. Chande"]},
    "chpbs": {"type":"course_manual","people":["Tushar S. Chande"]},
  },
} satisfies AuthorFile;
