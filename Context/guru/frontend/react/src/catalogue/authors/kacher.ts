import type {AuthorFile} from '../author-files';

export default {
  order: 1270,
  author: { id: "kacher", name: "Chris Kacher & Gil Morales", tagline: "O'Neil disciples · pocket pivots, buyable gap-ups and the seven-week rule (judgement-based, held)", books: [
    { id: "kmtld", sourceIds: ["title-34c3189b69f02c8c", "title-2b9f0c289e337d32"], title: "Trade Like an O'Neil Disciple", year: 2010, blurb: "Morales & Kacher, 2010 · pocket pivots, gap-ups, 10/50-day sell guides, market direction model; held: leading-stock and base-quality judgement, model not disclosed" },
    { id: "kmitc", sourceIds: ["title-47b3251474394b71"], title: "In the Trading Cockpit with the O'Neil Disciples", year: 2013, blurb: "Kacher & Morales, 2013 · buyable gap-ups and the seven-week rule; held: gap size 'eyeballed', exit only" },
    { id: "kmssd", sourceIds: ["title-ce28056178d468a8"], title: "Short-Selling with the O'Neil Disciples", year: 2015, blurb: "Morales & Kacher, 2015 · short-sale chart patterns in former leaders; held: chart judgement" }] },
  identities: ["Chris Kacher","Gil Morales"],
  profile: {
    "kind": "person",
    "mastery": [
      "CAN SLIM-style growth-stock trading",
      "Pocket pivots and gap-up buy points"
    ],
    "summary": "Chris Kacher and Gil Morales worked as proprietary traders for William O'Neil + Co. Their books describe pocket pivots, buyable gap-ups and moving-average sell guides used in fundamentally strong leading stocks; the volume and price tests are stated, but stock and base quality and the sell guide are judged by the trader, and their market timing model is not disclosed, so nothing is run. This profile rests on the books themselves.",
    "known_for": [
      "Trade Like an O'Neil Disciple",
      "In the Trading Cockpit with the O'Neil Disciples"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "kmtld": {"type":"book","people":["Chris Kacher","Gil Morales"]},
    "kmitc": {"type":"book","people":["Chris Kacher","Gil Morales"]},
    "kmssd": {"type":"book","people":["Chris Kacher","Gil Morales"]},
  },
} satisfies AuthorFile;
