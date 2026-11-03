import type {AuthorFile} from '../author-files';

export default {
  order: 100,
  author: { id: 'cooper', name: 'Jeff Cooper', tagline: 'Short-term swing setups on momentum stocks', books: [{ id: 'jc5dm', title: 'The 5 Day Momentum Method', year: 1997, blurb: 'Cooper, 1997 · momentum buys and shorts with a five-day exit · IBD relative-strength filter needs data' }, { id: 'jchr1', title: 'Hit and Run Trading', year: 1996, blurb: 'Cooper, 1996 · most setups need chart judgement; the testable ones run here' }, { id: 'jchr2', title: 'Hit and Run Trading II', year: 1998, blurb: 'Cooper, 1998 · no complete rule runs on our data' }] },
  identities: ["Jeff Cooper"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term swing",
      "Day trading",
      "Momentum"
    ],
    "summary": "Jeff Cooper began his career at Drexel Burnham's Beverly Hills office and created the Hit and Run methodology of short-term stock trading. Since 1996 he has published a market-timing newsletter and nightly swing and day-trade ideas based on fixed rules.",
    "known_for": [
      "Hit and Run Trading",
      "Hit and Run Trading II",
      "The 5 Day Momentum Method"
    ],
    "sources": [
      "https://cooperhitandruntrading.com/about/",
      "https://www.thestreet.com/opinion/out-of-the-dark-with-hit-and-run-trading-10028460"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "jc5dm": {"type":"book","people":["Jeff Cooper"]},
    "jchr1": {"type":"book","people":["Jeff Cooper"]},
    "jchr2": {"type":"book","people":["Jeff Cooper"]},
  },
} satisfies AuthorFile;
