import type {AuthorFile} from '../author-files';

export default {
  order: 2754,
  author: { id: "summa", name: "John Summa", tagline: "Options and sentiment · the Squeeze Play on the VIX", books: [
    { id: "tacsp", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/John F.Summa - Trading Against the Crowd"], title: "Trading Against the Crowd", year: 2004, blurb: "Summa, 2004 · Squeeze Play I: a VIX EMA5-21 oscillator cross with a price trigger, exit on the EMA21-50 cross; run on NIFTY 50 with India VIX as a labelled adaptation, readings declared, unverified" }] },
  identities: ["John Summa"],
  profile: {
    "kind": "person",
    "mastery": ["Options", "Market sentiment"],
    "summary": "John Summa wrote Trading Against the Crowd (2004) on sentiment-based trading; this profile rests on that book alone.",
    "known_for": ["Trading Against the Crowd"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "tacsp": {"type":"book","people":["John Summa"]},
  },
} satisfies AuthorFile;
