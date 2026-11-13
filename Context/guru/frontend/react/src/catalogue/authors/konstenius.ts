import type {AuthorFile} from '../author-files';

export default {
  order: 2450,
  author: { id: "konstenius", name: "Jeremy G. Konstenius", tagline: "S&C 1996 · trend filter plus regression-slope oscillator on weekly futures", books: [
    { id: "konwv", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C12/COMBINI.pdf"], title: "Combining Trend And Oscillator Signals (S&C, 1996)", year: 1996, blurb: "Konstenius · wave 1 model: weekly close vs its average plus a regression-slope oscillator vs its trigger line; yen 30/5/50 and T-bond 20/10/30 on Norgate continuous futures" }] },
  identities: ["Jeremy G. Konstenius"],
  sourceReview: {
    "konwv": {"type":"article","people":["Jeremy G. Konstenius"]},
  },
} satisfies AuthorFile;
