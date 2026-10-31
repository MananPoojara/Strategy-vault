import type {AuthorFile} from '../author-files';

export default {
  order: 940,
  author: { id: "hyerczyk", name: "James A. Hyerczyk", tagline: "Gann analyst and futures market writer · the swing-chart stop and reverse runs here", books: [
    { id: "jhppt", sourceIds: ["title-f4dad71e89ea05e4"], title: "Pattern, Price & Time: Using Gann Theory in Trading Systems (2nd ed.)", year: 2009, blurb: "Hyerczyk · Gann 1-, 2- and 3-bar swing charts; buy on a stop over the last swing top, sell and reverse under the last swing bottom, the stop following each new swing" }] },
  identities: ["James A. Hyerczyk"],
  profile: {
    "kind": "person",
    "mastery": [
      "Gann analysis"
    ],
    "summary": "James A. Hyerczyk is a market analyst who writes on W.D. Gann's methods. Pattern, Price & Time explains Gann swing charts (1-, 2- and 3-bar trend indicators) and trades them with a buy stop over the last swing top and a sell-and-reverse stop under the last swing bottom. This profile rests on the book itself.",
    "known_for": [
      "Pattern, Price & Time"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jhppt": {"type":"book","people":["James A. Hyerczyk"]},
  },
} satisfies AuthorFile;
