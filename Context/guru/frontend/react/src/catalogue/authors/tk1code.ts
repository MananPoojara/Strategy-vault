import type {AuthorFile} from '../author-files';

export default {
  order: 1190,
  author: { id: 'tk1code', name: "MetaStock & TradeStation list code", tagline: "Equis PS sample systems from the NIKSAN MetaStock pack and the Success System list post", books: [
    { id: "nkms", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/@Misc/- Studies - No Sorted/[sm] Metastock Plugin - NIKSAN's MS Formula All/NIKSAN's MSFormula3/MS72PRFT.DTA"], title: "NIKSAN MetaStock pack: Equis PS systems (10-Day Breakout to Fractal Trading System 2)", year: null, blurb: "37 Equis PS System Tester records; 33 run as stored (breakouts, range switches, Aroon, bull/bear power, CCI, CMO, Elliott oscillator, fractals and more); the three Dynamic Momentum Index systems are held (MetaStock never printed that indicator's formula) and Consolidation Breakout awaits an engine fix" },
    { id: "nkms2", sourceIds: [], title: "NIKSAN MetaStock pack: Equis PS systems, part 2 (Landis Reversal to Volume Trading System)", year: null, blurb: "Equis PS System Tester records 54-96, run from the stored formulas; those needing an unpublished Equis indicator or a futures limit-move amount are held" },
    { id: "nkms3", sourceIds: [], title: "NIKSAN MetaStock pack: PS RSI Trailing Exit", year: null, blurb: "One Equis PS record, kept apart because its trades depend on whether an opposite Enter reverses the position (default: it does)" },
    { id: "tlsuc", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/_Misc/Success System (For TS)/SuccessSystem.doc"], title: "Success System (anonymous TradeStation list post)", year: null, blurb: "High above the 36-day EMA after the 5-day RSI drops 30 points from a 9-day peak above 70; buy stop at the high, true-range stops, out on the 6th day. The author's report: FTSE futures 1984-98, 251 trades, 63% winners; the $3,500 money stop is left out" }] },
  sourceReview: {
    "nkms": {"type":"code_package","people":[]},
    "nkms2": {"type":"code_package","people":[]},
    "nkms3": {"type":"code_package","people":[]},
    "tlsuc": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
