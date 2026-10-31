import type {AuthorFile} from '../author-files';

export default {
  order: 1030,
  author: { id: "lstephens", name: "Lank Stephens", tagline: "Futures day and position trader · the 4 Bar Fact and THE CLINCHER run here", books: [
    { id: "lsfb", sourceIds: [], title: "Trading the 4 Bar Fact & Calculating THE CLINCHER", year: 2007, blurb: "Stephens · a close beyond the bars 2 and 4 back flips the position at the close (only reversals count); the square root of high x low turning the other way gives tomorrow's one-way trade from the open to the close" }] },
  identities: ["Lank Stephens"],
  profile: {
    "kind": "person",
    "mastery": [
      "Futures trading",
      "Price patterns"
    ],
    "summary": "Lank Stephens wrote Trading the 4 Bar Fact & Calculating THE CLINCHER (2nd edition, 2007), a tutorial on a four-bar close reversal pattern traded on futures from 1-minute to daily charts and a daily one-direction bias grid built from the square root of each day's high times low. This profile rests on the tutorial itself.",
    "known_for": [
      "Trading the 4 Bar Fact & Calculating THE CLINCHER"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "lsfb": {"type":"book","people":["Lank Stephens"]},
  },
} satisfies AuthorFile;
