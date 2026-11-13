import type {AuthorFile} from '../author-files';

export default {
  order: 240,
  author: { id: 'krutsinger', name: 'Joe Krutsinger', tagline: 'Packaged mechanical systems · Buy Monday, RainMaker and the Secrets of the Masters', books: [{ id: 'jkbwts', title: 'Building Winning Trading Systems (seminar manual)', year: 1997, blurb: 'Krutsinger, 1997 · Dinapoli5, Gettess30, Buy Monday-1400, Saidenberg30, RainMaker' }, { id: 'jk36k', title: '$36,000 Worth of Trading Systems?', year: 1997, blurb: 'Krutsinger, 1997 · JK1 Buy Monday; JK20–31 code is not readable' }, { id: 'jkbk', title: 'Product pack (Boredom Killer, Dealer’s Choice)', year: 2008, blurb: 'Buy Monday 2008 code, SAFE worksheet, JK-Call Buyer' }, { id: 'jkzap', title: 'ZAP seminar slides', year: 2002, blurb: 'Ideas only, no complete rule' }, { id: 'jkppnt', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Joe Krutsinger - Position Point for TS  (joekrut.com)/Open Code.rar"], title: "Position Point 2003 (code)", year: 2003, blurb: "Pullback stop entries around the 40-bar average with 10-bar walk exits · $740 stop held" }, { id: "jkrngr", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Joe Krutsinger - Ranger for TS (joekrut.com)/Open Code.rar"], title: "Ranger (Krutsinger, TradeStation code)", year: null, blurb: "Stop 0.6 range beyond the close after a 2-bar comparison; the code's time exits cannot act, so stop and reverse" }, { id: "jkqtd", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Joe Krutsinger - Queen for TS (joekrut.com)/Open Code.rar", "USB HDD 1/TradeStation Systems & Indicators/Joe Krutsinger - Traders Delight for TS  (joekrut.com)/Open Code.rar"], title: "Queen and Traders Delight (open TradeStation code)", year: null, blurb: "Two day-trade stop systems: Queen buys a stop at yesterday's close after a down close; Traders Delight buys the high after a close below two days ago; the $300 stop is held" }] },
  identities: ["Joe Krutsinger"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Futures",
      "Day trading"
    ],
    "summary": "Joe Krutsinger is a CTA and trading-system designer who wrote his first system in 1976. He consulted for traders and firms, turning their ideas into computer-tested mechanical systems, and wrote books on mechanical strategies for index futures, forex, commodities and equities.",
    "known_for": [
      "The Trading Systems Toolkit",
      "Mechanical system packages and seminars"
    ],
    "sources": [
      "https://www.trade2win.com/members/joekrut.16757/about",
      "https://www.amazon.com/Trading-Systems-Toolkit-Money-Making-Futures/dp/1557385343",
      "https://www.vantagepointsoftware.com/news/new-twist-on-an-old-indicator/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "jkbwts": {"type":"course_manual","people":["Joe Krutsinger"]},
    "jk36k": {"type":"code_package","sameWorkAs":"jkbwts","people":["Joe Krutsinger"]},
    "jkbk": {"type":"code_package","people":["Joe Krutsinger"]},
    "jkzap": {"type":"course_manual","people":["Joe Krutsinger"]},
    "jkppnt": {"type":"code_package","people":["Joe Krutsinger"]},
    "jkrngr": {"type":"code_package","people":["Joe Krutsinger"]},
    "jkqtd": {"type":"code_package","people":["Joe Krutsinger"]},
  },
} satisfies AuthorFile;
