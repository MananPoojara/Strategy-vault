import type {AuthorFile} from '../author-files';

export default {
  order: 1460,
  author: { id: 'schwager', name: 'Jack D. Schwager', tagline: 'Market Wizards interviewer and author of the classic futures and technical-analysis guides · original systems, generic trend systems and his automated-systems course', books: [
    { id: 'jscgf', sourceIds: ['source-9842023f20128977', 'source-2321fb15450f11f3'], title: 'A Complete Guide to the Futures Markets', year: 2017, blurb: 'Schwager, 2nd ed. 2017 (1st ed. 1984) · wide-ranging day and run-day systems, moving-average, crossover, breakout, confirmation and countertrend systems' },
    { id: 'jsgsta', sourceIds: ['title-8cf757e7e5f22cc4'], title: 'Getting Started in Technical Analysis', year: 1999, blurb: 'Schwager, 1999 · the same generic systems with this book’s examples, plus MACD crossover' },
    { id: 'jsats', sourceIds: ['source-1662498244810a1a'], title: "Jack Schwager's Guide to Winning with Automated Trading Systems", year: null, blurb: 'Course and lesson manuals · 17 EasyLanguage systems run as printed; pyramids and dollar stops not run' },
    { id: 'sofmt', sourceIds: ['source-e5669e53d1f93b70'], title: 'Schwager on Futures: Managed Trading', year: 1996, blurb: 'Schwager, 1996 · the BARRA/MLM 12-month trend index rule; the rest is fund evaluation' },
    { id: 'nmwbk', sourceIds: ['source-4bbc3c54803e21b7', 'source-36a9a6329441408b', 'source-56a19ec030b3f2ff'], title: 'The New Market Wizards', year: 1992, blurb: 'Schwager, 1992 · interviews; Gil Blake’s one-penny fund-switching rule is the only complete rule' }] },
  identities: ["Jack D. Schwager"],
  profile: {
    "kind": "person",
    "mastery": [
      "Futures",
      "Trend following",
      "Trading systems"
    ],
    "summary": "Jack D. Schwager is a US futures analyst and fund manager, best known for the Market Wizards interview books. He also wrote A Complete Guide to the Futures Markets and the Schwager on Futures series, which describe generic trend systems and his own wide-ranging-day and run-day systems.",
    "known_for": [
      "Market Wizards",
      "A Complete Guide to the Futures Markets",
      "Schwager on Futures"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Jack_D._Schwager"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "jscgf": {"type":"book","people":["Jack D. Schwager","Mark Etzkorn"]},
    "jsgsta": {"type":"book","people":["Jack D. Schwager"]},
    "jsats": {"type":"course_manual","people":["Jack D. Schwager"]},
    "sofmt": {"type":"book","people":["Jack D. Schwager"]},
    "nmwbk": {"type":"book","people":["Jack D. Schwager"]},
  },
} satisfies AuthorFile;
