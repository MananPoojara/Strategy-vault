import type {AuthorFile} from '../author-files';

export default {
  order: 1930,
  author: { id: "bdormeier", name: "Buff Dormeier", tagline: "Volume-weighted averages · the 5/20-day VWMA crossover runs here", books: [
    { id: "bdvwma", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Las Vegas 2005 Traders Expo Presentation CD/AutoPlay/Docs/Dormeier Handout.pdf"], title: "Buff Up Your Moving Averages (Las Vegas Traders Expo 2005 handout)", year: 2005, blurb: "Dormeier · 5-day vs 20-day volume-weighted average crossover, long and short; simple averages as his control test; the VPCI version is held" }] },
  identities: ["Buff Dormeier"],
  sourceReview: {
    "bdvwma": {"type":"other","people":["Buff Dormeier"]},
  },
} satisfies AuthorFile;
