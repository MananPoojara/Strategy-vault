import type {AuthorFile} from '../author-files';

export default {
  order: 400,
  author: { id: 'weinstein', name: 'Stan Weinstein', tagline: 'Stage analysis on the 30-week average', books: [{ id: 'swsp', title: 'Secrets for Profiting in Bull and Bear Markets', year: 1988, blurb: 'Weinstein, 1988 · fund switching on the 30-week MA reproduces his dated Dow calls; stage reading is judgement' }] },
  identities: ["Stan Weinstein"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Market timing",
      "Breadth"
    ],
    "summary": "Stan Weinstein is a US technical analyst who published The Professional Tape Reader newsletter from 1972 to 2000. His single book describes Stage Analysis on weekly charts, in which each stock is in one of four stages relative to its 30-week moving average, together with a 'weight of the evidence' set of market indicators.",
    "known_for": [
      "Secrets for Profiting in Bull and Bear Markets (1988)",
      "Stage Analysis",
      "30-week moving average"
    ],
    "sources": [
      "https://store.traders.com/stcov392cowi.html",
      "https://www.nytimes.com/1983/11/20/business/the-tape-tells-all.html",
      "https://kb.bullcharts.com.au/knowledge-base/using-the-author-strategies/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "swsp": {"type":"book","people":["Stan Weinstein"]},
  },
} satisfies AuthorFile;
