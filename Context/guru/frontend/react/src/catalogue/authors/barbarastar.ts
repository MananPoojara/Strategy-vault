import type {AuthorFile} from '../author-files';

export default {
  order: 2605,
  author: {
    id: 'barbarastar',
    name: 'Barbara Star',
    tagline: 'Technical analysis writer · oscillator studies',
    books: [{
      id: 'bstar',
      sourceIds: [],
      title: 'Oscillator Variations (article · Stocks & Commodities, April 1993, pp. 174–177)',
      year: 1993,
      blurb: 'Star · article on price-excess and momentum oscillators, based on John Navarte’s 1985 “Reading between the lines”. The 24/12 crossover is a declared long/cash application; it is not a reproduction of a published trade study.',
    }],
  },
  identities: ['Barbara Star'],
  profile: {
    kind: 'person',
    mastery: ['Oscillators', 'Technical analysis writing'],
    summary: 'Barbara Star wrote “Oscillator Variations” for the April 1993 issue of Stocks & Commodities. The article develops oscillator examples from John Navarte’s 1985 “Reading between the lines”.',
    known_for: ['Oscillator Variations'],
    sources: [
      'https://store.traders.com/-v11-c04-oscilla-pdf.html',
      'https://store.traders.com/-v03-c06-read-pdf.html',
    ],
    confidence: 'high',
  },
  sourceReview: {
    bstar: {type: 'article', people: ['Barbara Star']},
  },
} satisfies AuthorFile;
