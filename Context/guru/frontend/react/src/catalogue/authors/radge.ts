import type {AuthorFile} from '../author-files';

export default {
  order: 30,
  author: { id: 'radge', name: 'Nick Radge', tagline: 'Australian systematic trader · breakout and day-trade rules from his 2012 book and trading course', books: [{ id: 'radge', title: 'Successful Stock Trading & Trading Course', year: 2012, blurb: 'Radge, 2012 · 2/20 moving-average breakout, open-to-close day trades with point stops · chart patterns not testable' }] },
  identities: ["Nick Radge"],
  profile: {
    "kind": "person",
    "mastery": [
      "Momentum",
      "Trend following",
      "System design & testing"
    ],
    "summary": "Nick Radge is an Australian trader (since 1985) and head of trading and research at The Chartist, a stock advisory service he and Trish Radge built from a 1998 education business. He trades mainly momentum and trend-following systems and teaches system design and coding.",
    "known_for": [
      "Stock momentum and trend-following systems",
      "The Chartist",
      "Trading System Mentor Course"
    ],
    "sources": [
      "https://cmtassociation.org/presenter/nick-radge/",
      "https://www.thechartist.com.au/how-did-you-become-a-stock-trader/",
      "https://www.thechartist.com.au/images/The_Chartist_Bio_2014.pdf"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "radge": {"type":"book","people":["Nick Radge"]},
  },
} satisfies AuthorFile;
