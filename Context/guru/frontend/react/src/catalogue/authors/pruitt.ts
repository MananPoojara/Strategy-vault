import type {AuthorFile} from '../author-files';

export default {
  order: 140,
  author: { id: 'pruitt', name: 'George Pruitt & John Hill', tagline: 'Futures Truth · complete TradeStation systems with printed code', books: [{ id: 'pbwts', title: 'Building Winning Trading Systems with TradeStation', year: 2003, blurb: 'Pruitt & Hill, 2003 · King Keltner, Bollinger Bandit, Thermostat, Dynamic Break Out II, Ghost Trader' }, { id: 'putg', title: 'The Ultimate Trading Guide', year: 2000, blurb: 'Hill, Pruitt & Hill, 2000 · turnkey systems #1-#3 and #5, plus the chapter-3 pattern studies' }] },
  identities: ["George Pruitt","John Hill"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Futures"
    ],
    "summary": "John Hill founded Futures Truth in 1985 to independently test and track commercial trading systems; George Pruitt worked there for 31 years as a programmer and director of research. Together they wrote books on building and testing mechanical systems, much of it in TradeStation EasyLanguage.",
    "known_for": [
      "Building Winning Trading Systems with TradeStation",
      "The Ultimate Trading Guide",
      "Futures Truth system ratings"
    ],
    "sources": [
      "https://georgepruitt.com/georges-biography/",
      "http://traders.com/documentation/feedbk_docs/2003/01/Abstracts_new/Interview/interview.html",
      "https://futurestruth.com/about/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "pbwts": {"type":"book","people":["George Pruitt","John R. Hill"]},
    "putg": {"type":"book","people":["George Pruitt","John R. Hill","Lundy Hill"]},
  },
} satisfies AuthorFile;
