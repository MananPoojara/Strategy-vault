import type {AuthorFile} from '../author-files';

export default {
  order: 530,
  author: { id: 'sperandeo', name: "Victor Sperandeo & David Vomund", tagline: "Moving-average tests, 1-2-3 / 2B, ETF rotation", books: [{ id: 'svtvc', title: "Trader Vic on Commodities (Sperandeo)", year: 2008, blurb: "Sperandeo, 2008 · S&P 150/200/250-day tests reproduce 15–19 of 22 printed years; 1-2-3 and 2B need chart judgement" }, { id: 'svmws', title: "Methods of a Wall Street Master (Sperandeo)", year: 1991, blurb: "Sperandeo, 1991 · 10/30-week cross; 1-2-3 and 2B need chart judgement" }, { id: 'svtv2', title: "Trader Vic II (Sperandeo)", year: 1994, blurb: "Sperandeo, 1994 · nothing testable without chart judgement" }, { id: 'dvetf', title: "ETF Trading Strategies Revealed (Vomund)", year: 2007, blurb: "Vomund, 2007 · every model rotates ETFs the vault does not have" }] },
  identities: ["David Vomund","Victor Sperandeo"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Market timing",
      "Futures"
    ],
    "summary": "Victor 'Trader Vic' Sperandeo is a US trader who traded mainly energy and metals and later developed indexes. He was profiled in The New Market Wizards. His books cover trend identification, Dow Theory and the 1-2-3 trend-change rule. David Vomund is an AIQ analyst who writes about ETF timing and selection.",
    "known_for": [
      "Methods of a Wall Street Master",
      "1-2-3 trend-change rule",
      "ETF Trading Strategies Revealed (Vomund)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Victor_Sperandeo",
      "http://www.visalert.com/sub-pages/about-us.htm"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "svtvc": {"type":"book","people":["Victor Sperandeo"]},
    "svmws": {"type":"book","people":["Victor Sperandeo"]},
    "svtv2": {"type":"book","people":["Victor Sperandeo"]},
    "dvetf": {"type":"book","people":["David Vomund"]},
  },
} satisfies AuthorFile;
