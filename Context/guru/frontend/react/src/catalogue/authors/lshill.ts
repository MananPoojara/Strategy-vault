import type {AuthorFile} from '../author-files';

export default {
  order: 2756,
  author: { id: "lshill", name: "Lundy Stafford Hill", tagline: "Futures Truth Magazine · the 200-day average as an overbought indicator", books: [
    { id: "ftma", sourceIds: [], readerPaths: ["Trading Knowledge I/@Newsletters/Futures Truth Magazine (2007 - 2010) (futurestruth.com) (issue 5-2009)"], title: "The 200 Day Moving Average as an Overbought Indicator (Futures Truth 5-2009)", year: 2009, blurb: "Hill, 2009 · buy after a close a set percentage above the 200-day average and go flat at it; S&P futures table printed, run on SPY and the NSE indices, not reproduced, readings declared, unverified" }] },
  identities: ["Lundy Stafford Hill"],
  profile: {
    "kind": "person",
    "mastery": ["Futures trading systems"],
    "summary": "Lundy Stafford Hill wrote a 2009 Futures Truth Magazine article testing the 200-day moving average as an overbought indicator; this profile rests on that article alone.",
    "known_for": ["Futures Truth Magazine contributions"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ftma": {"type":"article","people":["Lundy Stafford Hill"]},
  },
} satisfies AuthorFile;
