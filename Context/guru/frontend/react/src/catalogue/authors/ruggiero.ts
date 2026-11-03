import type {AuthorFile} from '../author-files';

export default {
  order: 260,
  author: { id: 'ruggiero', name: 'Murray Ruggiero', tagline: 'Cybernetic trading · calendar, gap and breakout studies; intermarket systems need data', books: [{ id: 'rgcts', title: 'Cybernetic Trading Strategies', year: 1997, blurb: 'Ruggiero, 1997 · day-of-week, trading-day-of-month, runaway gaps, channel and volatility breakouts' }, { id: 'rgtsec', title: 'Traders’ Secrets (with Toghraie)', year: 1999, blurb: 'Ruggiero’s analysis sections' }, { id: 'rgstt', title: 'Stock Traders Toolbox', year: 2001, blurb: 'Manual leaves out the parameters; code is encoded' }, { id: 'rgsh', title: 'SimpleHarmony (Futures Truth review)', year: 2011, blurb: 'Review discloses no rules' }] },
  identities: ["Murray Ruggiero"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Quantitative research",
      "Market timing"
    ],
    "summary": "Murray A. Ruggiero Jr. is a developer of computerised market-timing systems, known for applying neural networks, genetic algorithms and intermarket analysis to trading. He was vice president of R&D at TradersStudio, president of Ruggiero Associates, and patented a method for embedding a neural network in a spreadsheet.",
    "known_for": [
      "Cybernetic Trading Strategies (1997)",
      "Intermarket and neural-network trading models",
      "TradersStudio"
    ],
    "sources": [
      "https://www.wiley.com/en-us/Cybernetic+Trading+Strategies%3A+Developing+a+Profitable+Trading+System+with+State-of-the-Art+Technologies-p-9780471149200",
      "https://www.moneyshow.com/expert/251ac8a996334999a1d8038da8aa3194/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "rgcts": {"type":"book","people":["Murray Ruggiero"]},
    "rgtsec": {"type":"book","people":["Adrienne Laris Toghraie","Murray Ruggiero"]},
    "rgstt": {"type":"code_package","people":[]},
    "rgsh": {"type":"article","people":["George Pruitt","Murray Ruggiero"]},
  },
} satisfies AuthorFile;
