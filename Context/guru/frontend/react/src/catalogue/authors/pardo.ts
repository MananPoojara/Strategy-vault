import type {AuthorFile} from '../author-files';

export default {
  order: 220,
  author: { id: 'pardo', name: 'Robert Pardo', tagline: 'System design, testing and walk-forward optimisation', books: [{ id: 'rpdt', title: 'Design, Testing and Optimization of Trading Systems', year: 1992, blurb: 'Pardo, 1992 · 5/20 SMA crossover, 125% range breakout, 3-day vs 12-day midpoint MA' }, { id: 'rpeo', title: 'The Evaluation and Optimization of Trading Strategies', year: 2008, blurb: 'Pardo, 2008 · EOTS 10/30 MA from the book’s code, 125%/145% range stop-and-reverse' }] },
  identities: ["Robert Pardo"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Quantitative research"
    ],
    "summary": "Robert (Bob) Pardo is president of Pardo Capital Management and an early developer of algorithmic trading-system testing. His 1991 book Design, Testing and Optimization of Trading Systems, expanded as The Evaluation and Optimization of Trading Strategies, set out a step-by-step process that includes optimization and walk-forward analysis.",
    "known_for": [
      "Walk-forward analysis",
      "Design, Testing and Optimization of Trading Systems",
      "The Evaluation and Optimization of Trading Strategies"
    ],
    "sources": [
      "https://cmtassociation.org/presenter/robert-pardo/",
      "https://www.wiley.com/en-us/The+Evaluation+and+Optimization+of+Trading+Strategies%2C+2nd+Edition-p-9780470128015",
      "https://onlinelibrary.wiley.com/doi/book/10.1002/9781119196969"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "rpdt": {"type":"book","people":["Robert Pardo"]},
    "rpeo": {"type":"book","sameWorkAs":"rpdt","people":["Robert Pardo"]},
  },
} satisfies AuthorFile;
