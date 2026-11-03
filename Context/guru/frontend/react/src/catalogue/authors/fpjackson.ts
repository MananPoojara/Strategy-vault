import type {AuthorFile} from '../author-files';

export default {
  order: 1040,
  author: { id: "fpjackson", name: "Franklin Paul Jackson", tagline: "Spice Market Institute · the I-S Method's Key MA-1 runs here", books: [
    { id: "fpjis", sourceIds: [], title: "Studies in Market Action and Speculative Technique: The I-S Method", year: null, blurb: "Jackson · 10-week average plotted three weeks ahead; a weekly stop one-fourth of its square root beyond it gets you out, and the average turning the same way completes the reversal (Pre signals alone as an option)" }] },
  identities: ["Franklin Paul Jackson"],
  profile: {
    "kind": "person",
    "mastery": [
      "Weekly stock charting",
      "Moving averages"
    ],
    "summary": "Franklin Paul Jackson wrote the I-S Method course (Studies in Market Action and Speculative Technique, Spice Market Institute), which builds trend reversal 'keys' from square-root swing distances and a 10-week moving average advanced three weeks on weekly stock charts. This profile rests on the course itself.",
    "known_for": [
      "The I-S Method"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fpjis": {"type":"book","people":["Franklin Paul Jackson"]},
  },
} satisfies AuthorFile;
