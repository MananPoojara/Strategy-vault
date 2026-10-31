import type {AuthorFile} from '../author-files';

export default {
  order: 1830,
  author: { id: "kyuan", name: "Kathy Yuan, Lu Zheng & Qiaoqiao Zhu", tagline: "Finance researchers \u00b7 the lunar-phase trading strategy from 'Are Investors Moonstruck?'", books: [
    { id: "yzzlp", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Kathy Yuan - Lunar Phases and Stock Returns (Article)/Lunar Phases and Stock Returns.pdf"], title: "Are Investors Moonstruck? Lunar Phases and Stock Returns (2001)", year: 2001, blurb: "Yuan, Zheng & Zhu \u00b7 long in the 15 days around new moons, short around full moons; the paper's 4.2% is a 48-country portfolio" }] },
  identities: ["Kathy Yuan", "Lu Zheng", "Qiaoqiao Zhu"],
  sourceReview: {
    "yzzlp": {"type":"article","people":["Kathy Yuan","Lu Zheng","Qiaoqiao Zhu"]},
  },
} satisfies AuthorFile;
