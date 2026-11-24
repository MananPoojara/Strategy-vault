import type {AuthorFile} from '../author-files';

export default {
  order: 2790,
  author: { id: "primeline", name: "Prime-Line", tagline: "Prime9 · 27 SMA cross on 9-minute bars, run here on index minute data as a declared adaptation", books: [
    { id: "prime9", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Prime Line Prime9 System. Harmonic Aligment Scalp Strategy (prime-line.com)/Harmonic Alignment Scalp Strategy.pdf"], title: "Prime9 System - Harmonic Alignment Scalp Strategy (Prime-Line)", year: 2007, blurb: "Prime-Line slides · buy when price crosses above the 27-period SMA on a 9-minute chart while the line is flat or rising (sell short mirrored), exit on the cross back; no initial stop distance is printed so none is applied; runs on SPX / NIFTY 50 / NIFTY BANK 9-minute bars from 1-minute data as a declared adaptation, no trade list printed, unverified" }] },
  identities: ["Prime-Line"],
  profile: {
    "kind": "organization",
    "mastery": ["Short-term scalping", "Moving-average cross rules"],
    "summary": "Prime-Line (prime-line.com) sold the Prime9 'Harmonic Alignment' scalp strategy for futures on a 9-minute chart, built on a 27-period simple average and the number 9.",
    "known_for": ["Prime9 System"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "prime9": {"type":"course_manual","people":["Prime-Line"]},
  },
} satisfies AuthorFile;
