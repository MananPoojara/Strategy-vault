import type {AuthorFile} from '../author-files';

export default {
  order: 470,
  author: { id: 'freeburg', name: "Nelson Freeburg", tagline: "Formula Research timing models and workshop systems", books: [
    { id: "nfce5", sourceIds: [], title: "CD Handouts (Chicago 2005 Traders Expo)", year: 2005, blurb: "Freeburg · the Allen and Traub weekly S&P 500 timing models; the other eight models need yields, CRB or the discount rate" },
{ id: 'nftm', title: "Timing Models & Classic Indicators", year: 2002, blurb: "Freeburg, 2002 · AccuTrack two-index switch; bond and intermarket models need data" }, { id: 'nfhp', title: "Building High-Performance Trading Systems", year: 2000, blurb: "Freeburg, 2000 · Quadram countertrend and trend-following on NYSE and NSE breadth" }, { id: 'nfgu', title: "Building a Mechanical Trading System from the Ground Up", year: 1996, blurb: "Freeburg, 1996 · Pathfinder, Channel 25, CQIII and PAL need futures or bond data" }, { id: 'nfmt', title: "Mechanical Timing Systems", year: 1999, blurb: "Freeburg, 1999 · Quadram with a T-Bond filter needs bond data" }, { id: 'nfpk', title: "Trading the Pankin Strategy", year: 1997, blurb: "Freeburg, 1997 · the weekly MA crossovers he illustrates; Pankin’s own rules are never printed" }] },
  identities: ["Nelson Freeburg"],
  profile: {
    "kind": "person",
    "mastery": [
      "Market timing",
      "System design & testing",
      "Indicators"
    ],
    "summary": "Nelson Freeburg is the Memphis-based editor and publisher of the Formula Research newsletter. Each issue presents a mechanical trading or market-timing system, often built from classic indicators, tested over long historical periods.",
    "known_for": [
      "Formula Research newsletter",
      "Timing Models & Classic Indicators",
      "Pankin Strategy test"
    ],
    "sources": [
      "https://technical.traders.com/Products/display.asp?dbname=publication%5Cpublication&prodid=37&tablename=Publication",
      "https://tradingmarkets.com/recent/nelson_freeburg_on_developing_the_perfect_trading_system_part_ii-664409",
      "https://store.traders.com/-v14-c10-modelin-pdf.html"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "nfce5": {"type":"course_manual","people":["Nelson Freeburg"]},
    "nftm": {"type":"course_manual","people":["Nelson Freeburg"]},
    "nfhp": {"type":"course_manual","people":["Nelson Freeburg"]},
    "nfgu": {"type":"course_manual","people":["Nelson Freeburg"]},
    "nfmt": {"type":"course_manual","people":["Nelson Freeburg"]},
    "nfpk": {"type":"course_manual","people":["Nelson Freeburg"]},
  },
} satisfies AuthorFile;
