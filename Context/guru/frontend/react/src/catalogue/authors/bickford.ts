import type {AuthorFile} from '../author-files';

export default {
  order: 1350,
  author: { id: "bickford", name: "James L. Bickford", tagline: "Forex charting and wave theory · swing-chart wave forecasts and activity charts", books: [
    { id: "jlbcmf", sourceIds: ["title-34c72bf5ac6f798c"], title: "Charting the Major Forex Pairs (with Archer)", year: 2007, blurb: "Archer & Bickford · annual, monthly and composite charts; no trading rule" },
    { id: "jlbfwt", sourceIds: ["title-a05ecdb0b6af70ca"], title: "Forex Wave Theory", year: 2007, blurb: "Bickford, 2007 · wave-cycle forecasts on swing charts; held: no entry or exit" },
    { id: "jlbfsa", sourceIds: ["title-23a1d123324dac61"], title: "Forex Shockwave Analysis", year: 2008, blurb: "Bickford, 2008 · activity, range and wave forecasts; held: forecasts only" }] },
  identities: ["James L. Bickford"],
  profile: {
    "kind": "person",
    "mastery": [
      "Forex charting",
      "Wave analysis"
    ],
    "summary": "James L. Bickford is a software engineer and forex analyst who co-wrote several forex books with Michael D. Archer. His books in the library chart the major currency pairs and forecast wave sizes on swing charts; they give no entry or exit rules, so nothing is tested. This profile rests on the books themselves.",
    "known_for": [
      "Forex Wave Theory",
      "Getting Started in Currency Trading (with Archer)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jlbcmf": {"type":"book","people":["James L. Bickford"]},
    "jlbfwt": {"type":"book","people":["James L. Bickford"]},
    "jlbfsa": {"type":"book","people":["James L. Bickford"]},
  },
} satisfies AuthorFile;
