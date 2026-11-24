import type {AuthorFile} from '../author-files';

export default {
  order: 2704,
  author: { id: "foreximpact", name: "Forex Impact (Sharptrade Partners)", tagline: "Forex Gap Play · the weekly gap fade runs here on index daily bars as a declared adaptation", books: [
    { id: "fxgap", sourceIds: [], readerPaths: ["Trading Knowledge I/Jason Fielder - Forex Profit Formula System (forexprofitformula.com)/Gap Trading the Forex/Gap Trading the Forex.pdf"], title: "Forex Gap Play (Forex Impact)", year: 2008, blurb: "Forex Impact · fade a weekly open gap of 15 pips or more with two lots, stop 150 pips, lot 1 out at the previous week's close, lot 2 at the week's end with a breakeven stop after the retrace; runs on NIFTY 50 / NIFTY BANK daily bars (1 pip = 0.01 percent) as a declared adaptation, no trade list printed; results withheld: the stop, the retrace and the breakeven stop fall in the same daily bar too often (needs minute data)" } ,
    { id: "fxfs", sourceIds: [], readerPaths: ["Trading Knowledge I/Joel Risnick - First Strike Plus, First Strike Plus & One Night Stand (infiniteyield.com)"], title: "FirstStrike (Joel Rensink)", year: 2008, blurb: "Rensink / Forex Impact folder · weekly OCO: Monday open +/- 50 pips buy/sell stop, 60-pip stop, exit at the Friday close; runs on SPX / NIFTY 50 / NIFTY BANK 1-minute bars (1 pip = 0.01 percent) as a declared adaptation; the file's GBP/USD figures (1,275 trades, 45% winners) are not reproduced on indices" }] },
  identities: ["Forex Impact"],
  profile: {
    "kind": "collection",
    "mastery": ["Forex gap trading", "Forex Impact system guides"],
    "summary": "Forex Impact (Sharptrade Partners) sold short forex system guides. This profile rests on the Forex Gap Play guide (pages 3-19 have a text layer and pages 20-33 were OCRed; all were read); other Forex Impact guides are judged separately.",
    "known_for": ["Forex Gap Play"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fxgap": {"type":"course_manual","people":["Forex Impact"]},
    "fxfs": {"type":"course_manual","people":["Joel Rensink"]},
  },
} satisfies AuthorFile;
