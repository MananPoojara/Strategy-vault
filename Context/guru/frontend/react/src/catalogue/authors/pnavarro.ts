import type {AuthorFile} from '../author-files';

export default {
  order: 1950,
  author: { id: "pnavarro", name: "Peter Navarro", tagline: "Macrowave investing · the weekly 3-point-break method runs here", books: [
    { id: "pnwmm", sourceIds: [], readerPaths: ["Trading Books/Peter Navarro - When the Market Moves, Will You Be Ready/Peter Navarro - When The Market Moves, Will You Be Ready.pdf"], title: "When the Market Moves, Will You Be Ready?", year: 2004, blurb: "Navarro · 3-point-break on weekly closes: long at the 3rd higher high, short at the 3rd lower low, out when the trend breaks (Table 8-2 QQQ reproduced)" }] },
  identities: ["Peter Navarro"],
  sourceReview: {
    "pnwmm": {"type":"book","people":["Peter Navarro"]},
  },
} satisfies AuthorFile;
