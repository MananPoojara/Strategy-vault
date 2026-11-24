import type {AuthorFile} from '../author-files';

export default {
  order: 2742,
  author: { id: "kotze", name: "Ian Kotze", tagline: "J9S1 · RSI + Chande oscillator on 10-minute bars, run here on index minute data as a declared adaptation", books: [
    { id: "j9s1", sourceIds: [], readerPaths: ["Trading Knowledge I/Surefire Trading Challenge Winning Trading Systems (surefiretradingchallenge.com)/June 2009/The J9S1 System/j9s1.pdf"], title: "The J9S1 System (Ian Kotze)", year: 2009, blurb: "Kotze, June 2009 Surefire challenge · buy when the 9-period RSI is below 30 and the 13-period Chande Momentum Oscillator below -50 on a 10-minute chart (sell above 70 / 50), fixed 200-point stop and 350-point target scaled to 0.91 / 1.59 percent; runs on SPX / NIFTY 50 / NIFTY BANK 10-minute bars from 1-minute data as a declared adaptation, no trade list printed, unverified" }] },
  identities: ["Ian Kotze"],
  profile: {
    "kind": "person",
    "mastery": ["Short-term scalping", "RSI and Chande momentum rules"],
    "summary": "Ian Kotze (South Africa, day trader) took first place in the June 2009 Surefire challenge with the J9S1 system, a 10-minute RSI(9) and Chande Momentum(13) overbought/oversold system on the ALSI Top 40. This profile rests on that 7-page report (read in full).",
    "known_for": ["The J9S1 System"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "j9s1": {"type":"course_manual","people":["Ian Kotze"]},
  },
} satisfies AuthorFile;
