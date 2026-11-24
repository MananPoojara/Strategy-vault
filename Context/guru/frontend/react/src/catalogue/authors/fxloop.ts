import type {AuthorFile} from '../author-files';

export default {
  order: 2791,
  author: { id: "fxloop", name: "The Forex Loophole (forexloophole.com)", tagline: "Daily three-close exhaustion fade · USD/CAD rule run on inverted Canadian dollar futures (results withheld: bar-path dependent)", books: [
    { id: "fxloop", sourceIds: [], readerPaths: ["USB HDD 2/Trading Knowledge 3/Forex Loop Hole (forexloophole.com)/thefxloop.pdf"], title: "The Forex Loophole", year: 2008, blurb: "forexloophole.com guide · nightly set-up on three closes against last Friday's close, next-day 7-pip stop entry, 60/40-pip target/stop on USD/CAD; run on the exactly inverted Norgate Canadian dollar series; withheld (17.1% of trades depend on the bar path)" },
  ], },
  identities: ["The Forex Loophole"],
  profile: {
    "kind": "collection",
    "mastery": ["Daily forex exhaustion fade"],
    "summary": "A forex guide sold as 'The Forex Loophole' (pdf, about 6k words, read in full on 25 Nov 2026): one nightly set-up and entry rule with per-pair pip targets and stops; no results are printed.",
    "known_for": ["Forex Loophole"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fxloop": {"type":"course_manual","people":["The Forex Loophole"]},
  },
} satisfies AuthorFile;
