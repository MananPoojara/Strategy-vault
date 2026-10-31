import type {AuthorFile} from '../author-files';

export default {
  order: 1599,
  author: { id: "jsilva", name: "Jose Silva", tagline: "MetaStock programmer (metastocktools.com) · two of his teaching examples run here, labelled as such", books: [
    { id: "jsursc", sourceIds: [], title: "URSC3 toolkit for MetaStock: system development articles", year: 2006, blurb: "Silva, 2006 · price breakout and 'do not trade' contrarian breakout examples, run as the author's demonstrations" }] },
  identities: ["Jose Silva"],
  profile: {
    "kind": "person",
    "mastery": [
      "MetaStock programming",
      "System testing"
    ],
    "summary": "Jose Silva runs metastocktools.com and wrote the URSC3 MetaStock toolkit; its 2006 system development articles use simple breakout rules as teaching examples. This profile rests on those articles alone.",
    "known_for": [
      "URSC3 toolkit for MetaStock"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jsursc": {"type":"article","people":["Jose Silva"]},
  },
} satisfies AuthorFile;
