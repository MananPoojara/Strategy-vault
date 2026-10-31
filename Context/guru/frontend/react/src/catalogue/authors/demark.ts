import type {AuthorFile} from '../author-files';

export default {
  order: 200,
  author: { id: 'demark', name: 'Tom DeMark', tagline: 'TD indicators · Setup, TD Lines, REBO and short-term patterns', books: [{ id: 'tdperl', title: 'DeMark Indicators', year: 2008, blurb: 'Perl, 2008 · perfected setups, TD Lines, REBO, Differential/Clop/Clopwin · Sequential and Combo 13 have no exit in the book, so they are not tested' }, { id: 'tdnsta', title: 'The New Science of Technical Analysis', year: 1994, blurb: 'DeMark, 1994 · TD Sequential with DeMark’s own exit (p.172-178)' }, { id: 'tdnmtt', title: 'New Market Timing Techniques', year: 1997, blurb: 'DeMark, 1997 · Sequential, Combo I/II and TD Trap with the book’s exits (p.102, 131)' }, { id: 'tdwb', title: "Trading Indicators for the 21st Century — workbooks", year: null, blurb: "DeMark · disqualified TD Line break faded, exit given by the workbook" }, { id: 'tdscal', title: "TD Scalper code (Simms)", year: null, blurb: "Simms, built on DeMark’s setup · needs 5-minute bars" }] },
  identities: ["Tom DeMark"],
  profile: {
    "kind": "person",
    "mastery": [
      "Market timing",
      "Indicators"
    ],
    "summary": "Tom DeMark began at the pension fund National Investment Services researching investment timing and developed his own proprietary timing models. He consulted for many large funds, has advised Steven A. Cohen for over 27 years, and founded DeMARK Analytics. The CMT Association gave him its Annual Award in 2020.",
    "known_for": [
      "DeMARK Indicators (TD Sequential, TD Combo)",
      "The New Science of Technical Analysis",
      "New Market Timing Techniques"
    ],
    "sources": [
      "https://demark.com/about/",
      "https://demark.com/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "tdperl": {"type":"book","people":["Jason Perl"]},
    "tdnsta": {"type":"book","people":["Tom DeMark"]},
    "tdnmtt": {"type":"book","people":["Tom DeMark"]},
    "tdwb": {"type":"course_manual","people":["Tom DeMark"]},
    "tdscal": {"type":"code_package","people":["Mark A. Simms"]},
  },
} satisfies AuthorFile;
