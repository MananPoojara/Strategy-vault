import type {AuthorFile} from '../author-files';

export default {
  order: 2100,
  author: { id: "asinghal", name: "Aseem Singhal", tagline: "51 Trading Strategies · NSE index, stock and option set-ups; 9 run here (daily and index minute bars), 42 held with the reason", books: [
    { id: "as51ts", sourceIds: [], readerPaths: ["Trading Books/51- Trading Strategies/Trading Strategies - All Images.pdf"], title: "51 Trading Strategies", year: 2023, blurb: "Singhal, ZebraLearn 2023 · TradingView set-ups on NIFTY, BANKNIFTY and NSE stocks: Ichimoku, fractals, Donchian, Supertrend, RSI, scalps and weekly-option trades; worked examples, no performance tables" }] },
  identities: ["Aseem Singhal"],
  sourceReview: {
    "as51ts": {"type":"book","people":["Aseem Singhal"]},
  },
} satisfies AuthorFile;
