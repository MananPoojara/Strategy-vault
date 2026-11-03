import type {AuthorFile} from '../author-files';

export default {
  order: 550,
  author: { id: 'mtiming', name: "Masonson, Siegel, Lydon & James", tagline: "Moving-average and calendar market-timing models", books: [{ id: 'masmt', title: "All About Market Timing (Masonson)", year: 2004, blurb: "Masonson, 2004 · NDX 6% swing, 50/200, September and presidential-cycle rules; midterm Nov–Apr matches 4 of 4 years" }, { id: 'sglr', title: "Stocks for the Long Run (Siegel)", year: 2002, blurb: "Siegel, 3rd ed. 2002 · 200-day average with a 1% band; his 1987 and 2001 Dow exits match" }, { id: 'lydetf', title: "The ETF Trend Following Playbook (Lydon)", year: null, blurb: "Lydon · 200-day EMA trend rule" }, { id: 'fejmma', title: "Monthly Moving Averages (F.E. James)", year: null, blurb: "James · monthly average rules, long/short per his walk-through" }, { id: 'lydpes', title: "Profitable ETF Strategies (Lydon & Wasik)", year: null, blurb: "Lydon & Wasik · 200-day average with an 8% exit and 5% re-entry" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Market timing",
      "Trend following",
      "Momentum"
    ],
    "summary": "Long-horizon timing and trend-following for index funds and ETFs. It ranges from Jeremy Siegel's Wharton research on long-run stock returns to monthly moving-average and ETF trend playbooks.",
    "known_for": [
      "Stocks for the Long Run (Siegel)",
      "Monthly Moving Averages (F.E. James)",
      "The ETF Trend Following Playbook (Lydon)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Jeremy_Siegel"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "masmt": {"type":"book","people":["Leslie N. Masonson"]},
    "sglr": {"type":"book","people":["Jeremy J. Siegel"]},
    "lydetf": {"type":"book","people":["Tom Lydon"]},
    "fejmma": {"type":"article","people":["F. E. James"]},
    "lydpes": {"type":"book","people":["John F. Wasik","Tom Lydon"]},
  },
} satisfies AuthorFile;
