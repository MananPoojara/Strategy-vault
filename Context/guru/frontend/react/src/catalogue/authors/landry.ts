import type {AuthorFile} from '../author-files';

export default {
  order: 290,
  author: { id: 'landry', name: 'Dave Landry', tagline: 'Swing trading · pullbacks, knockouts and bow ties with exact triggers', books: [{ id: 'dlost', title: 'Dave Landry on Swing Trading', year: 2001, blurb: 'Landry, 2001 · Simple Pullback, Trend Knockout, Bow Tie, Oscillator Swing System, CVR III-Modified' }, { id: 'dlstfl', title: 'Swing Trading for a Living (workbook)', year: 2002, blurb: 'Landry, 2002 · Close Count and the VIX High/Low system' }, { id: 'dl10b', title: "10 Best Swing Trading Patterns", year: 2003, blurb: "Landry, 2003 · Explosion Gap Pivots and Reversal Gap; the rest need trend judgement" }] },
  identities: ["Dave Landry"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term swing",
      "Pattern recognition",
      "Trend following"
    ],
    "summary": "Dave Landry has traded since the early 1990s and founded the trading and consulting firm Sentive Trading in 1995. He teaches pattern-based swing trading of stocks, with emphasis on waiting for proper entries, money management and cutting losses.",
    "known_for": [
      "Dave Landry on Swing Trading (2000)",
      "10 Best Swing Trading Patterns & Strategies (2003)",
      "The Layman's Guide to Trading Stocks (2010)"
    ],
    "sources": [
      "https://www.davelandry.com/about-dave-landry/",
      "https://stockcharts.com/articles/bios/dave-landry.html"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "dlost": {"type":"book","people":["Dave Landry"]},
    "dlstfl": {"type":"course_manual","people":["Dave Landry"]},
    "dl10b": {"type":"book","people":["Dave Landry"]},
  },
} satisfies AuthorFile;
