import type {AuthorFile} from '../author-files';

export default {
  order: 270,
  author: { id: 'momentum', name: 'Antonacci, AQR & others', tagline: 'Momentum and gap research · Dual Momentum, time-series momentum, opening gaps', books: [{ id: 'gadm', title: 'Dual Momentum Investing (Antonacci)', year: 2014, blurb: 'GEM needs T-bills, a bond index and ACWI ex-US: no data, nothing substituted' }, { id: 'aqrtf', title: 'A Century of Evidence on Trend-Following (AQR)', year: 2012, blurb: 'Hurst, Ooi & Pedersen · 1-, 3- and 12-month time-series momentum and the combination' }, { id: 'lpgap', title: 'Opening Gap Strategy (Leslie Payne)', year: 2007, blurb: 'Payne · gap fade to yesterday’s close with a 95% stop' }, { id: 'jmls', title: 'The L/S Model Trading System (Murakami)', year: 2002, blurb: 'All systems need intraday bars' }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Momentum",
      "Trend following",
      "Quantitative research"
    ],
    "summary": "A collection of momentum and trend research plus two short-term systems. It holds Gary Antonacci's Dual Momentum Investing, which combines relative-strength and absolute (trend) momentum across stock and bond indices, and AQR's century-long study of trend following. It also includes Leslie Payne's Opening Gap Strategy and Murakami's L/S Model Trading System.",
    "known_for": [
      "Dual Momentum (Antonacci)",
      "A Century of Evidence on Trend-Following (AQR)",
      "Opening Gap Strategy (Payne)",
      "L/S Model Trading System (Murakami)"
    ],
    "sources": [
      "https://www.optimalmomentum.com/about-gary-antonacci-dual-momentum-investing-expert/",
      "https://www.oreilly.com/library/view/dual-momentum-investing/9780071849449/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "gadm": {"type":"book","people":["Gary Antonacci"]},
    "aqrtf": {"type":"article","people":["Brian Hurst","Lasse H. Pedersen","Yao Hua Ooi"]},
    "lpgap": {"type":"code_package","people":["Leslie Payne"]},
    "jmls": {"type":"book","people":["Josh Murakami"]},
  },
} satisfies AuthorFile;
