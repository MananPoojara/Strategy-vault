/* Recorded source evidence; no simulated prices or book claims used as strategy stats. */
window.ChandeEvidence = {
  "review_date": "2026-10-09",
  "scope": "Initial65sma-3cc and CB-PB source audit; supplied partial edition, no full-book certification. Two OHLC research adaptations; three futures-stop variants withheld.",
  "basis": "OHLC market adaptation using selected observed cash-index, ETF or stock prices and return-based costs. Original one-contract futures studies, dollar P&L, roll construction and intraday risk are not reproduced. Cash-index shorts are hypothetical without borrow, financing or futures basis. The supplied1997 book is partial.",
  "methods": {
    "ch_65sma3cc": {
      "source": {
        "path": "Books/Tushar Chande/Tushar Chande - Beyond Technical Analysis. How to Develop and Implement a Winning Trading System.pdf",
        "sha256": "18476d76fdc2882fb9d5a1b134de45935198553a11f99628e27cb6ecdf3da18f"
      },
      "verified_performance": false,
      "source_study_reproduced": false,
      "source_gaps": [
        "Supplied book ends at printed143; later chapters and bibliography/index are missing."
      ],
      "pages": "printed75\u201381 / PDF91\u201397",
      "conclusion": "Baseline65-day SMA and three contemporaneous closes; next-open opposite reversal, no merged stops. Independent execution diagnostics match, while the original23-market futures study is unreproduced. The source558USD is a market-average aggregation, not pooled577.81USD per trade.",
      "reason": "Selected-market OHLC adaptation; original futures contract/roll inputs and author ledger are missing.",
      "external": [
        "https://catalogimages.wiley.com/images/db/pdf/0471415677.pdf"
      ],
      "missing_data": [
        "Original continuous-futures input series, roll construction and author trade ledger for source-study reproduction."
      ],
      "rule_gaps": []
    },
    "ch_cbpb_time": {
      "source": {
        "path": "Books/Tushar Chande/Tushar Chande - Beyond Technical Analysis. How to Develop and Implement a Winning Trading System.pdf",
        "sha256": "18476d76fdc2882fb9d5a1b134de45935198553a11f99628e27cb6ecdf3da18f"
      },
      "verified_performance": false,
      "source_study_reproduced": false,
      "source_gaps": [
        "Supplied book ends at printed143; later chapters and bibliography/index are missing."
      ],
      "pages": "printed101\u2013111 / PDF122\u2013134; code PDF125, trailing units PDF128, tables PDF131\u2013133",
      "conclusion": "Preserve rolling HighestBar semantics and most-recent ties. Yesterday-window ages0\u20136 correspond to current-bar ages1\u20137. Fixed-horizon adaptation follows stated exit prose with entry age0; supplied code visibly uses a minus sign, so literal code parity is not claimed. Table4.8 average280USD disagrees with displayed market averages136.71USD and pooled152.54USD. Original trade outcomes are unavailable.",
      "external": [
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/function/highestbar_function_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/barssinceentry_reserved_word_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/point_reserved_word_.htm"
      ],
      "reason": "Selected-market fixed-horizon OHLC adaptation, not replication of the original futures study.",
      "missing_data": [
        "Original22-market continuous-contract quotes and source ledger for performance reproduction."
      ],
      "rule_gaps": []
    },
    "ch_cbpb_dollar": {
      "source": {
        "path": "Books/Tushar Chande/Tushar Chande - Beyond Technical Analysis. How to Develop and Implement a Winning Trading System.pdf",
        "sha256": "18476d76fdc2882fb9d5a1b134de45935198553a11f99628e27cb6ecdf3da18f"
      },
      "verified_performance": false,
      "source_study_reproduced": false,
      "source_gaps": [
        "Supplied book ends at printed143; later chapters and bibliography/index are missing."
      ],
      "pages": "printed101\u2013111 / PDF122\u2013134; code PDF125, trailing units PDF128, tables PDF131\u2013133",
      "conclusion": "Preserve rolling HighestBar semantics and most-recent ties. Yesterday-window ages0\u20136 correspond to current-bar ages1\u20137. Fixed-horizon adaptation follows stated exit prose with entry age0; supplied code visibly uses a minus sign, so literal code parity is not claimed. Table4.8 average280USD disagrees with displayed market averages136.71USD and pooled152.54USD. Original trade outcomes are unavailable.",
      "external": [
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/function/highestbar_function_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/barssinceentry_reserved_word_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/point_reserved_word_.htm"
      ],
      "reason": "Futures dollar-stop or contract-order implementation remains unsupported.",
      "missing_data": [
        "Dated contract point values, PriceScale/MinMove, real contract/roll history and dollar-stop conversion."
      ],
      "rule_gaps": [
        "Exact initial-stop, entry-day, gap and limit/stop precedence must be validated."
      ]
    },
    "ch_cbpb_target": {
      "source": {
        "path": "Books/Tushar Chande/Tushar Chande - Beyond Technical Analysis. How to Develop and Implement a Winning Trading System.pdf",
        "sha256": "18476d76fdc2882fb9d5a1b134de45935198553a11f99628e27cb6ecdf3da18f"
      },
      "verified_performance": false,
      "source_study_reproduced": false,
      "source_gaps": [
        "Supplied book ends at printed143; later chapters and bibliography/index are missing."
      ],
      "pages": "printed101\u2013111 / PDF122\u2013134; code PDF125, trailing units PDF128, tables PDF131\u2013133",
      "conclusion": "Preserve rolling HighestBar semantics and most-recent ties. Yesterday-window ages0\u20136 correspond to current-bar ages1\u20137. Fixed-horizon adaptation follows stated exit prose with entry age0; supplied code visibly uses a minus sign, so literal code parity is not claimed. Table4.8 average280USD disagrees with displayed market averages136.71USD and pooled152.54USD. Original trade outcomes are unavailable.",
      "external": [
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/function/highestbar_function_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/barssinceentry_reserved_word_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/point_reserved_word_.htm"
      ],
      "reason": "Futures dollar-stop or contract-order implementation remains unsupported.",
      "missing_data": [
        "Dated contract point values, PriceScale/MinMove, real contract/roll history and dollar-stop conversion."
      ],
      "rule_gaps": [
        "Exact initial-stop, entry-day, gap and limit/stop precedence must be validated."
      ]
    },
    "ch_cbpb_trail": {
      "source": {
        "path": "Books/Tushar Chande/Tushar Chande - Beyond Technical Analysis. How to Develop and Implement a Winning Trading System.pdf",
        "sha256": "18476d76fdc2882fb9d5a1b134de45935198553a11f99628e27cb6ecdf3da18f"
      },
      "verified_performance": false,
      "source_study_reproduced": false,
      "source_gaps": [
        "Supplied book ends at printed143; later chapters and bibliography/index are missing."
      ],
      "pages": "printed101\u2013111 / PDF122\u2013134; code PDF125, trailing units PDF128, tables PDF131\u2013133",
      "conclusion": "Preserve rolling HighestBar semantics and most-recent ties. Yesterday-window ages0\u20136 correspond to current-bar ages1\u20137. Fixed-horizon adaptation follows stated exit prose with entry age0; supplied code visibly uses a minus sign, so literal code parity is not claimed. Table4.8 average280USD disagrees with displayed market averages136.71USD and pooled152.54USD. Original trade outcomes are unavailable.",
      "external": [
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/function/highestbar_function_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/barssinceentry_reserved_word_.htm",
        "https://help.tradestation.com/10_00/eng/tsdevhelp/elword/word/point_reserved_word_.htm"
      ],
      "reason": "Futures dollar-stop or contract-order implementation remains unsupported.",
      "missing_data": [
        "Dated contract point values, PriceScale/MinMove, real contract/roll history and dollar-stop conversion."
      ],
      "rule_gaps": [
        "Exact initial-stop, entry-day, gap and limit/stop precedence must be validated.",
        "Source code1 Point differs from prose1 tick unless MinMove=1; no automatic -1.0 or ratcheting stop is inferred."
      ]
    }
  },
  "books": [
    {
      "id": "ch_bta",
      "title": "Beyond Technical Analysis",
      "year": 1997,
      "blurb": "Partial supplied1997 edition; explicit OHLC adaptations and separate contract-data blockers."
    }
  ]
};
