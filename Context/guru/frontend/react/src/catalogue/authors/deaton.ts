import type {AuthorFile} from '../author-files';

export default {
  order: 1160,
  author: { id: "deaton", name: "Mark Deaton", tagline: "Renegade Trader · the signal and trigger swings run here", books: [
    { id: "mdstm", sourceIds: ["title-a0cd64c8be7bdfd8"], title: "Swing Trading Money Machine", year: 2007, blurb: "Deaton · a low, a lower low and a higher low make a swing; the next swing with a higher lower low is the trigger: buy as price passes its first bar's high, stop under its lower low, out on the 3-5 bar count (5 by default; 161.8% / 261.8% targets as options)" }] },
  identities: ["Mark Deaton"],
  profile: {
    "kind": "person",
    "mastery": [
      "Swing trading",
      "Price swings and Fibonacci targets"
    ],
    "summary": "Mark Deaton (Renegade Trader) wrote the Swing Trading Money Machine course (2007), which enters on 'signal' and 'trigger' price swings and exits on a 3-5 bar count or Fibonacci extensions, shown on ETF and index charts, plus candlestick and options workbooks. This profile rests on the course manual itself.",
    "known_for": [
      "Swing Trading Money Machine"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mdstm": {"type":"book","people":["Mark Deaton"]},
  },
} satisfies AuthorFile;
