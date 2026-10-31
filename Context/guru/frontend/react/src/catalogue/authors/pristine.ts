import type {AuthorFile} from '../author-files';

export default {
  order: 1580,
  author: {id:'pristine',name:'Pristine Trading',aliases:['prestine','pristine','Greg Capra','Oliver Velez','Oliver L. Velez'],tagline:'Greg Capra and Oliver Velez’s trading education, plus Pristine course material. Original documents are available; complete rule and execution review is pending.',books:[
    {id:'p_intraday_capra',title:'Intraday Trading Techniques',year:2001,author_credit:'Greg Capra',readerPaths:['Books/Greg Capra/Greg Capra - Intraday Trading Techniques.pdf'],blurb:'Pristine intraday techniques presented by Greg Capra. The source opening page establishes the author and 2001 copyright. Complete entry, exit, stop and execution rules still need independent review; no performance is attributed to this document.'},
    {id:'p_tools_tactics',title:'Tools & Tactics for the Master Day Trader',year:2000,author_credit:'Oliver Velez & Greg Capra',readerPaths:['Books/Greg Capra/Oliver Velez & Greg Capra - Tools & Tactics for the Master Day Trader.pdf'],publisher_source:'https://mheducation.com/highered/mhp/product/tools-tactics-master-day-trader-battle-tested-techniques-day-swing-position-traders.html',blurb:'Oliver Velez and Greg Capra’s guide to day, swing and position trading. McGraw Hill confirms the authors and 2000 first edition. A complete chapter and execution-rule review remains pending; the separate magazine strategy is not a verified implementation of this book.'},
    {id:'p_tpm1',title:'Trading the Pristine Method — Part 1',year:null,author_credit:'Pristine Capital Holdings; individual author unconfirmed',readerPaths:['Books/Greg Capra/TPM1 Live Color Manual.pdf'],blurb:'The scanned cover identifies Trading the Pristine Method, Part 1, from Pristine Capital Holdings. The individual author and edition year are unconfirmed. Read the original manual; mechanical rules and management decisions have not yet been independently extracted and validated.'}
  ]},
  identities: ["Greg Capra","Oliver Velez"],
  profile: {
    "kind": "collection",
    "mastery": [
      "Day trading",
      "Short-term swing",
      "Pattern recognition"
    ],
    "summary": "Pristine is a trading-education firm founded in 1994 by Oliver Velez, joined by Greg Capra. It ran one of the first online real-time trading rooms. The Pristine Method teaches intraday and swing setups based on candlestick bars, moving averages and support and resistance.",
    "known_for": [
      "The Pristine Method",
      "Tools & Tactics for the Master Day Trader"
    ],
    "sources": [
      "https://traders.com/documentation/feedbk_docs/2003/03/Abstracts_new/Interview/interview.html",
      "https://www.nytimes.com/1998/01/26/business/no-frills-just-service-with-a-screen.html"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "p_intraday_capra": {"type":"course_manual","people":["Greg Capra"]},
    "p_tools_tactics": {"type":"book","people":["Greg Capra","Oliver Velez"]},
    "p_tpm1": {"type":"course_manual","people":[]},
  },
} satisfies AuthorFile;
