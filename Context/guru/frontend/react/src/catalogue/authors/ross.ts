import type {AuthorFile} from '../author-files';

export default {
  order: 1520,
  author: { id: 'ross', name: 'Joe Ross', photo:'img/gurus/joe-ross.png', photoSource:'https://tradingeducators.com/component/k2/item/50?Itemid=512', get books() { return (window as any).JoeRossEvidence.books; } },
  identities: ["Joe Ross"],
  profile: {
    "kind": "person",
    "mastery": [
      "Pattern recognition",
      "Short-term swing",
      "Futures"
    ],
    "summary": "Joe Ross (died 2021) was a private trader who founded Trading Educators in 1988 and wrote twelve books. His chart-based approach defines tradable formations, the 1-2-3 high/low, ledge, trading range and Ross hook, under his Law of Charts, and enters early using the Traders Trick Entry.",
    "known_for": [
      "Ross hook",
      "The Law of Charts",
      "Traders Trick Entry",
      "Trading the Ross Hook"
    ],
    "sources": [
      "https://www.tradingeducators.com/private-mentoring/joe-ross"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "jr_manual": {"type":"book","people":["Joe Ross"]},
    "jr_hook": {"type":"book","people":["Joe Ross"]},
    "jr_loc": {"type":"article","people":["Joe Ross"]},
    "jr_tte": {"type":"course_manual","people":["Joe Ross"]},
    "jr_zulu": {"type":"course_manual","people":["Joe Ross"]},
    "jr_ambush": {"type":"course_manual","people":["Joe Ross"]},
    "jr_tbtb": {"type":"book","people":["Joe Ross"]},
    "jr_tnt1": {"type":"course_manual","people":["Joe Ross"]},
    "jr_tnt3": {"type":"course_manual","people":["Joe Ross"]},
  },
} satisfies AuthorFile;
