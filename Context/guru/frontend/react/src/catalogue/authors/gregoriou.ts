import type {AuthorFile} from '../author-files';

export default {
  order: 570,
  author: { id: 'gregoriou', name: 'Greg N. Gregoriou', tagline: 'Editor of academic volumes on hedge funds, CTAs and trading · the moving-average and breakout rules his contributors test', books: [
    { id: 'ggth', sourceIds: ['title-96a2d227d4a0091c'], title: 'The Handbook of Trading', year: 2010, blurb: 'Gregoriou (ed.), 2010 · (1,n) moving-average rule (Ch.7) and Lento’s traditional and smoothed dual-average crossover (Ch.11); portfolio and model chapters need data' },
    { id: 'ggeai', sourceIds: ['title-6d67ab8fb1faada6'], title: 'Encyclopedia of Alternative Investments', year: 2009, blurb: 'Gregoriou (ed.), 2009 · the Systematic CTA entry’s 5/30-day crossover example; the other entries are definitions and studies' },
    { id: 'gghf', sourceIds: ['title-754e28c0678483ee'], title: 'Hedge Funds: Insights in Performance Measurement, Risk Analysis, and Portfolio Allocation', year: 2005, blurb: 'Gregoriou et al. (eds.), 2005 · the Calyon 80-day breakout and 80-day average rules, 15-day momentum benchmark rule; fund-ranking chapters need fund data' },
    { id: 'ggfohf', sourceIds: ['title-d853521dd05ed147'], title: 'Funds of Hedge Funds', year: 2006, blurb: 'Gregoriou (ed.), 2006 · price vs 224/117/61/32-day average trend factor (Ch.4); the rest studies fund performance' }] },
  identities: ["Greg N. Gregoriou"],
  profile: {
    "kind": "person",
    "mastery": [
      "Hedge funds",
      "Managed futures",
      "Academic research"
    ],
    "summary": "Greg N. Gregoriou edits academic volumes on hedge funds, commodity trading advisors, mutual funds and trading. The rules on this card come from chapters written by his contributors (Kenourgios and Papathanasiou, Lento, Black, and the Calyon index study), not from Gregoriou himself; this profile rests on the books' title pages.",
    "known_for": [
      "The Handbook of Trading",
      "Encyclopedia of Alternative Investments",
      "Commodity Trading Advisors"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ggth": {"type":"book","people":["Greg N. Gregoriou"]},
    "ggeai": {"type":"book","people":["Greg N. Gregoriou"]},
    "gghf": {"type":"book","people":["Greg N. Gregoriou"]},
    "ggfohf": {"type":"book","people":["Greg N. Gregoriou"]},
  },
} satisfies AuthorFile;
