import type {AuthorFile} from '../author-files';

export default {
  order: 1830,
  author: { id: "yshramenko", name: "Yuri Shramenko", tagline: "Cycle forecaster (techedgecycles.com) · his Hi-Lo Bands entry and trailing-stop technique runs here, without his forecast filter", books: [
    { id: "ysthl", sourceIds: [], readerPaths: ["Trading Courses,Seminars,Videos/Yuri Shramenko - Trade Execution (techedgecycles.com)/Yuri Shramenko - Trade Execution.pdf"], title: "Trade Execution: Entry and Exit Examples (Hi-Lo Bands)", year: null, blurb: "Shramenko · enter on a penetration of the 8-day high or low band, trail the opposite band, then the near band once a whole bar clears it" }] },
  identities: ["Yuri Shramenko"],
  sourceReview: {
    "ysthl": {"type":"course_manual","people":["Yuri Shramenko"]},
  },
} satisfies AuthorFile;
