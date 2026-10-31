import type {AuthorFile} from '../author-files';

export default {
  order: 1210,
  author: { id: "jlcava", name: "José Luis Cava", tagline: "Spanish technical analyst · three systems from his book run here", books: [
    { id: "jcsis", sourceIds: [], title: "Sistemas de Especulación en Bolsa", year: 2001, blurb: "Cava · the 50-bar mid-line, volume and MBO three-indicator rule, the RSI + Momentum system and the Three-Bar Rule (Spanish, scanned)" }] },
  identities: ["José Luis Cava"],
  profile: {
    "kind": "person",
    "mastery": [
      "Technical analysis",
      "Spanish stock market"
    ],
    "summary": "José Luis Cava wrote Sistemas de Especulación en Bolsa (2001), a Spanish book on trading systems. This profile rests on the book itself.",
    "known_for": [
      "Sistemas de Especulación en Bolsa"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jcsis": {"type":"book","people":["José Luis Cava"]},
  },
} satisfies AuthorFile;
