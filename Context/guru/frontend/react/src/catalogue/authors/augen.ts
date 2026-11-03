import type {AuthorFile} from '../author-files';

export default {
  order: 1300,
  author: { id: "augen", name: "Jeff Augen", tagline: "Options trader · volatility analysis and expiration-day option trades (held)", books: [
    { id: "jaoth", sourceIds: ["title-2b121333ca367e1c"], title: "The Option Trader's Handbook", year: 2004, blurb: "Augen · option structures and adjustments; held" },
    { id: "javeo", sourceIds: ["title-d6656189ada520c1"], title: "The Volatility Edge in Options Trading", year: 2008, blurb: "Augen, 2008 · price-spike volatility analysis; held" },
    { id: "jatoe", sourceIds: ["title-4554d758de73b9a1", "title-cbbb8b5db21bbc5f"], title: "Trading Options at Expiration", year: 2009, blurb: "Augen, 2009 · expiration-day single-stock option trades on minute data; held" }] },
  identities: ["Jeff Augen"],
  sourceReview: {
    "jaoth": {"type":"book","people":["Jeff Augen"]},
    "javeo": {"type":"book","people":["Jeff Augen"]},
    "jatoe": {"type":"book","people":["Jeff Augen"]},
  },
} satisfies AuthorFile;
