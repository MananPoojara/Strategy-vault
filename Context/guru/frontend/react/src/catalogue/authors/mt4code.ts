import type {AuthorFile} from '../author-files';

export default {
  order: 2030,
  author: { id: "mt4code", name: "MetaTrader 4 expert advisors (open MQ4 code)", tagline: "Timeframe-free forex EAs run on daily bars as a declared application \u00b7 pip stops and targets held", books: [
    { id: "mt4ec", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/_Forexmentor.com/Alan Benefield - Mastering Metatrader 4 in 90 Minutes & Members Site/BONUS 1 - Metatrader Toolkit/EMA_CROSS.mq4"], title: "EMA_CROSS (Coders Guru, Forexmentor MetaTrader toolkit)", year: null, blurb: "10/80 EMA direction change, flat to open and the next change to close; as printed it buys when the 10 EMA falls under the 80 (the usual order is an option)" },
    { id: "mqlc08", sourceIds: [], readerPaths: ["USB HDD 1/META TRADER/MetaTrader/33 Experts from 2008 MQL Championship/2008.mq4"], title: "2008.mq4 (Sergejs Bogdanovs, 2008 MQL Championship)", year: 2008, blurb: "RSI(14) back up through 40 buys, back down through 60 sells, stop and reverse at the next open" }] },
  sourceReview: {
    "mt4ec": {"type":"code_package","people":["Coders Guru"]},
    "mqlc08": {"type":"code_package","people":["Sergejs Bogdanovs"]},
  },
} satisfies AuthorFile;
