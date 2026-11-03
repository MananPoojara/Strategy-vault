import type {AuthorFile} from '../author-files';

export default {
  order: 1280,
  author: { id: "bortucene", name: "Erol Bortucene", tagline: "Forex day-trading courses (daytradeforex.com) · pip-based intraday systems, held", books: [
    { id: "ebdtf", sourceIds: ["title-4d2f69fb3c8c5e7b", "title-3b8f399beb3e7b35", "title-c7b2a15f91962795", "title-49874d42323ea583", "title-32890ff3ede78152"], title: "The Day Trade Forex System (with Cynthia Macy)", year: null, blurb: "Bortucene & Macy · 5-minute currency scalps with pip targets; held" },
    { id: "ebeft", sourceIds: ["title-1293eb6b88467d85", "title-01ada73be922ee55"], title: "The Euro Fractal Trading System", year: null, blurb: "Bortucene · EUR/USD key-level fractal breakouts, 15-pip stops; held" },
    { id: "ebfps", sourceIds: ["title-6fde110723e9f0ba"], title: "The Forex Profit System", year: null, blurb: "Bortucene · hourly EMA 10/25/50 cross with 15-minute SAR; held: stop and SAR settings not fixed" }] },
  identities: ["Erol Bortucene"],
  sourceReview: {
    "ebdtf": {"type":"book","people":["Erol Bortucene","Cynthia Macy"]},
    "ebeft": {"type":"book","people":["Erol Bortucene"]},
    "ebfps": {"type":"book","people":["Erol Bortucene"]},
  },
} satisfies AuthorFile;
