import type {AuthorFile} from '../author-files';

export default {
  order: 600,
  author: { id: "fontanills", name: "George A. Fontanills", tagline: "Optionetics founder and options educator · Volex bands, ROC/ADX momentum, volume and moving-average ETF systems from his courses with Tom Gentile", books: [
    { id: "fgitc", sourceIds: ["title-2c7a4a19e234e1ab"], title: "The Index Trading Course", year: 2006, blurb: "Fontanills & Gentile, 2006 · Volex bands, ROC/ADX momentum, QQQQ volume system, 10/20 EMA ETF system; seasonal rules repeat Kaeppel" },
    { id: "fgitsm", sourceIds: ["title-a07933c3570be209"], title: "Interactive Technical Trading Seminar (manual)", year: null, blurb: "Optionetics seminar manual · Reverse VolEx and volume-spike systems; Elliott-wave trades are proprietary" },
    { id: "fgsmc", sourceIds: ["title-516adf9252b9d55c"], title: "The Stock Market Course", year: 2001, blurb: "Fontanills & Gentile, 2001 · 50/200-day crossover; the VIX rule uses the old OEX index" },
    { id: "fgtofd", sourceIds: ["title-7310501fb0b6fb77"], title: "Trading Options For Dummies", year: 2008, blurb: "Fontanills, 2008 · 20/50-day EMA crossover example; option strategies and the sector tilt need data" }] },
  identities: ["George A. Fontanills"],
  profile: {
    "kind": "person",
    "mastery": [
      "Options",
      "Index trading",
      "Technical analysis"
    ],
    "summary": "George A. Fontanills founded the Optionetics trading-education company and wrote option and index trading courses, several with Tom Gentile. Most of his books teach option strategies (which need option prices); the Index Trading Course and the seminar manuals also state complete stock and ETF timing systems. This profile rests on the books themselves.",
    "known_for": [
      "The Options Course",
      "The Index Trading Course",
      "Trading Options For Dummies"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fgitc": {"type":"book","people":["George A. Fontanills"]},
    "fgitsm": {"type":"book","people":["George A. Fontanills"]},
    "fgsmc": {"type":"book","people":["George A. Fontanills"]},
    "fgtofd": {"type":"book","people":["George A. Fontanills"]},
  },
} satisfies AuthorFile;
