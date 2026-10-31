import type {AuthorFile} from '../author-files';

export default {
  order: 80,
  author: { id: 'kaufman', name: 'Perry Kaufman', tagline: 'Trading systems encyclopaedia · trend, momentum, breakout and adaptive methods', books: [{ id: 'kftsm', title: 'New Trading Systems and Methods (4th ed.)', year: 2005, blurb: 'Kaufman, 2005 · moving-average, breakout, efficiency-ratio (KAMA) and other systems from the book and its CD code' }, { id: 'kfsc', title: 'A Short Course in Technical Trading', year: 2003, blurb: 'Kaufman, 2003' }, { id: 'kfbpm', title: 'The Best (Public) Systems I Have Known (TAG workshop)', year: 1999, blurb: 'Kaufman, 1999 · 8-day MA countertrend, 2-Systems trend/countertrend and TSM Divergence from the printed code' }, { id: 'kfst', title: 'Smarter Trading', year: 1995, blurb: 'Kaufman, 1995 · exponential trend with profit-taking or stop-loss, adaptive moving average (Box 8-5)' }, { id: 'kfcd', title: "Trading Systems & Methods — companion CD code", year: 2005, blurb: "Kaufman, 4th ed. 2005 · FC Orson low-volatility breakout and the High/Low Volume and Volume Filters programs" }, { id: 'kfgei', title: "Global Equity Investing (Vivanti & Kaufman)", year: null, blurb: "Vivanti & Kaufman · weekly breakout, average and January systems; System B’s printed exits don’t follow its own 7% rule" }] },
  identities: ["Perry Kaufman"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Trend following",
      "Quantitative research"
    ],
    "summary": "Perry J. Kaufman is an American systematic trader and former aerospace engineer, described as a leading expert in fully algorithmic trading. His Trading Systems and Methods, first published in 1978 and now in its sixth edition, is a standard reference on trading techniques and system testing.",
    "known_for": [
      "Trading Systems and Methods",
      "Kaufman Adaptive Moving Average (efficiency ratio)",
      "Smarter Trading"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Perry_J._Kaufman"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "kftsm": {"type":"book","people":["Perry Kaufman"]},
    "kfsc": {"type":"book","people":["Perry Kaufman"]},
    "kfbpm": {"type":"course_manual","people":["Perry Kaufman"]},
    "kfst": {"type":"book","people":["Perry Kaufman"]},
    "kfcd": {"type":"code_package","sameWorkAs":"kftsm","people":["Perry Kaufman"]},
    "kfgei": {"type":"book","people":["Alberto Vivanti","Perry Kaufman"]},
  },
} satisfies AuthorFile;
