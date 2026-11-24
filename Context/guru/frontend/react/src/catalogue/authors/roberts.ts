import type {AuthorFile} from '../author-files';

export default {
  order: 2810,
  author: { id: "roberts", name: "Douglas S. Roberts", tagline: "Follow the Fed · switch between large and small stocks on the real short-term interest rate", books: [
    { id: "ftfed", sourceIds: [], readerPaths: ["Trading Books/Douglas Roberts - Follow the Fed to Investment Success The Effortless Strategy for Beating Wall Street"], title: "Follow the Fed to Investment Success", year: 2008, blurb: "Wiley 2008 · Standard Strategy: monthly, small stocks when the compounded 12-month T-bill return is below 12-month inflation (negative real short rate), else large stocks, signal applied two months later; runs on SPY / IWM with FRED 3-month bill and CPI as a declared adaptation, price only, no printed figure compared, unverified" }] },
  identities: ["Douglas S. Roberts"],
  profile: {
    "kind": "person",
    "mastery": ["Macro-driven stock allocation", "Federal Reserve policy"],
    "summary": "Douglas S. Roberts (Channel Capital Research Institute) wrote Follow the Fed to Investment Success (2008), a monthly switch between large and small stocks driven by Federal Reserve interest-rate policy.",
    "known_for": ["Follow the Fed"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ftfed": {"type":"book","people":["Douglas S. Roberts"]},
  },
} satisfies AuthorFile;
