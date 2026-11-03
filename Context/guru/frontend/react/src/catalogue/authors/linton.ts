import type {AuthorFile} from '../author-files';

export default {
  order: 950,
  author: { id: "linton", name: "David Linton", tagline: "Ichimoku cloud chart analyst · the cloud stop and reverse runs here", books: [
    { id: "dlccs", sourceIds: ["title-488a6153eb6af4ce"], title: "Cloud Charts: Trading Success with the Ichimoku Technique", year: 2010, blurb: "Linton · buy when the close moves above the cloud after the last outside close was below it, sell and reverse when it closes below; signal delay of 1-5 days and the lagging-line version as options" }] },
  identities: ["David Linton"],
  profile: {
    "kind": "person",
    "mastery": [
      "Ichimoku cloud charts"
    ],
    "summary": "David Linton is a technical analyst. Cloud Charts (published by Updata, 2010) explains how the Ichimoku cloud is built and read, and back-tests a simple stop and reverse: buy when the close moves above the cloud, sell and reverse when it closes below, optionally waiting a few days to filter short breaches. This profile rests on the book itself.",
    "known_for": [
      "Cloud Charts"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "dlccs": {"type":"book","people":["David Linton"]},
  },
} satisfies AuthorFile;
