import type {AuthorFile} from '../author-files';

export default {
  order: 2830,
  author: { id: "vakkur", name: "Mark Vakkur", tagline: "Treasury yield trend as an S&P 500 switch", books: [
    { id: "vkyld", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C08/TREASUR.pdf"], title: "Treasury Bond Yields And The S&P 500 (S&C)", year: 1996, blurb: "Vakkur · out of stocks when last month's 30-year Treasury yield is 0.1 or more above its six-month average of month-end yields, else the S&P 500; four unstated details labelled, cash = flat price only; the article's 1950-1995 monthly figures cannot be reproduced on the vault window, unverified" }] },
  identities: ["Mark Vakkur"],
  sourceReview: {
    "vkyld": {"type":"article","people":["Mark Vakkur"]},
  },
} satisfies AuthorFile;
