import type {AuthorFile} from '../author-files';

export default {
  order: 410,
  author: { id: 'guppy', name: 'Daryl Guppy (formula collection)', tagline: 'guppytraders.com MetaStock formula index · mostly reader and TASC systems, not Guppy’s own', books: [{ id: 'dgmf', title: 'MetaStock Formula Index', year: null, blurb: '28 complete systems from the collection, run with MetaStock tester defaults; GMMA is chart reading' }, { id: 'dggw', title: "GMMA workbook", year: null, blurb: "Guppy · GMMA is chart reading, nothing testable" }, { id: 'dgdwb', title: "Darvas Trading WorkBook", year: null, blurb: "Guppy · modern Darvas box runs; the classic version’s volume test is never measured" }, { id: 'dgppt', title: "Precision Pattern Trading", year: null, blurb: "Guppy · chart patterns, nothing testable" }, { id: 'dgmtt', title: "Market Trading Tactics", year: 2000, blurb: "Guppy, 2000 · judgement-based tactics, nothing testable" }, { id: 'dgnl', title: "Newsletters 2000–2004", year: null, blurb: "Guppy newsletters · SPI-futures and share systems need data; the rest is chart reading" }] },
  identities: ["Daryl Guppy"],
  profile: {
    "kind": "collection",
    "mastery": [
      "Trend following",
      "Indicators",
      "Breakout"
    ],
    "summary": "Daryl Guppy is an Australian trader and author best known for the Guppy Multiple Moving Average (GMMA), which compares a short-term and a long-term group of moving averages to judge trend strength and character. This collection gathers his MetaStock formulas, GMMA and Darvas workbooks, pattern-trading books and 2000-2004 newsletters.",
    "known_for": [
      "Guppy Multiple Moving Average (GMMA)",
      "Darvas box trading",
      "Trend Trading"
    ],
    "sources": [
      "https://www.guppytraders.com/gmma-info"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "dgmf": {"type":"code_package","people":[]},
    "dggw": {"type":"course_manual","people":["Daryl Guppy"]},
    "dgdwb": {"type":"course_manual","people":["Daryl Guppy"]},
    "dgppt": {"type":"course_manual","people":["Daryl Guppy"]},
    "dgmtt": {"type":"book","people":["Daryl Guppy"]},
    "dgnl": {"type":"magazine_volume","people":["Daryl Guppy"]},
  },
} satisfies AuthorFile;
