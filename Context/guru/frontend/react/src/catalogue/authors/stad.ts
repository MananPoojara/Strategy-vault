import type {AuthorFile} from '../author-files';

export default {
  order: 160,
  author: { id: 'stad', name: 'Omega Research STAD Club', tagline: 'System Trading and Development Club · fully coded EasyLanguage systems', books: [{ id: 'stad', title: 'STAD Club, Volumes 1–4, 7 and 10', year: 1997, blurb: 'Omega Research, 1997–2000 · VolEx, Monday We Trade, VIX key reversals and the Vol 1–4 systems, from the printed code' }, { id: 'stad2', title: 'STAD Club, Volumes 5–13', year: 1999, blurb: 'Omega Research, 1998–2000 · Derivative MA, Red Rover, Night Train, LR Crossing and 58 more from the printed code' }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Indicators"
    ],
    "summary": "Omega Research's System Trading & Development (STAD) Club issued bulletins every other month from January 1998, each with trading strategies and articles coded in TradeStation EasyLanguage. Volume 13 revisited selected strategies from Volumes 1-12. The collection is a set of vendor-published example systems, not one author's method.",
    "known_for": [
      "STAD Club volumes",
      "EasyLanguage example strategies"
    ],
    "sources": [
      "https://forum.arabictrader.com/uploaded/1081_STAD13.pdf",
      "https://www.purebytes.com/archives/omega/2000/msg06787.html"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "stad": {"type":"code_package","people":[]},
    "stad2": {"type":"code_package","sameWorkAs":"stad","people":[]},
  },
} satisfies AuthorFile;
