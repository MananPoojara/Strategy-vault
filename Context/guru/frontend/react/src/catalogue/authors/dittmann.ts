import type {AuthorFile} from '../author-files';

export default {
  order: 710,
  author: { id: "dittmann", name: "Karl Dittmann", tagline: "Forex e-book author · short-term pip systems on 15- and 30-minute currency charts (held: no forex data, pip-sized exits)", books: [
    { id: "kdfnlt", sourceIds: ["title-00f8c0f9d6793707"], title: "Forex Never Lose Trade", year: 2009, blurb: "Dittmann, 2009 · GBP/USD 7-9 am range breakout; held: intraday forex, pip stops" },
    { id: "kdfus", sourceIds: ["title-2f2153842d6ebee0"], title: "Forex Unknown Secret", year: 2010, blurb: "Dittmann, 2010 · 30-minute start-bar reversal; held: intraday forex, pip thresholds" },
    { id: "kdsfp", sourceIds: ["title-7e6081991fd067d5"], title: "The Simplest Forex Pips System", year: 2010, blurb: "Dittmann, 2010 · inside-close candle with a trend filter; held: pip target and a trailing stop without a size" }] },
  identities: ["Karl Dittmann"],
  profile: {
    "kind": "person",
    "mastery": [
      "Forex trading"
    ],
    "summary": "Karl Dittmann sold short forex e-books through his own websites around 2009-2010. Each describes a short-term currency set-up on 15- or 30-minute charts with stops and targets measured in pips. This profile rests on the books themselves.",
    "known_for": [
      "Forex Never Lose Trade",
      "The Simplest Forex Pips System"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "kdfnlt": {"type":"book","people":["Karl Dittmann"]},
    "kdfus": {"type":"book","people":["Karl Dittmann"]},
    "kdsfp": {"type":"book","people":["Karl Dittmann"]},
  },
} satisfies AuthorFile;
