import type {AuthorFile} from '../author-files';

export default {
  order: 50,
  author: { id: 'tharp', name: 'Van Tharp', tagline: 'Trading psychology, system design and position sizing. Books, articles and course workbooks are indexed separately; original portfolio results are not reproduced.', books: [
    { id: 'tharp', title: 'Trade Your Way to Financial Freedom', year: 1999, blurb: 'Tharp, 1999 · random entry with a 3x volatility trailing stop and the 55/21-day breakout run on indices, ETFs and stocks (declared application); the futures portfolio studies are not reproduced' },
    { id: 'tharp_super_trader', title: 'Super Trader', year: 2009, readerPaths:['Books/Van Tharp/Van Tharp - Super Trader.pdf'],sourceIds:['source-f34bbee982033423'],blurb:'Trading psychology, business planning and system development. Its source system examples and twenty-day volatility description remain separate from Trade Your Way to Financial Freedom.' },
    { id: 'tharp_system_workbook', title: 'Systems Development Workbook', year: null, readerPaths:['Books/Van Tharp/Van Tharp - Systems Development Workbook.PDF'],sourceIds:['source-be82b8940bd02f17'],blurb:'A workbook for defining and evaluating trading systems. Entry exercises and stop components do not establish complete strategy exits; edition and full execution review remain separate.' }
  ] },
  identities: ["Brian June","D. R. Barton Jr.","Henry O. Pruden","John Burley","Steve Sjuggerud","Van Tharp"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trading psychology",
      "Position sizing & risk",
      "System design & testing"
    ],
    "summary": "Van K. Tharp, PhD, was a trading coach from 1982 and founder of the Van Tharp Institute. He is known for work on trader psychology, self-sabotage and system development, with a strong focus on position sizing as the main driver of results.",
    "known_for": [
      "Trade Your Way to Financial Freedom",
      "R-multiples and expectancy",
      "Position sizing"
    ],
    "sources": [
      "https://vantharpinstitute.com/about-van-tharp/",
      "https://aiqsystems.com/van2pop.htm"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "tharp": {"type":"book","people":["Van K. Tharp"]},
    "tharp_super_trader": {"type":"book","people":["Van K. Tharp"]},
    "tharp_system_workbook": {"type":"course_manual","people":["Van K. Tharp"]},
  },
} satisfies AuthorFile;
