# Rule-fidelity audit, 20-21 Oct 2026 (candidate defects; nothing here is fixed)

Independent agents re-read each book first, wrote its rules down from the book alone, and only then compared our catalogue and code. About 440 catalogue entries were compared: Long-Term Secrets (lwlts), Kaeppel (kpss), Street Smarts (ss), S&P Market Timing (spmt), Ruggiero (rgcts), High Probability ETF (hpetf), Katz (kets), Collins (acbff), Stridsman (strtw), plus the shared MetaStock engine and the MetaStock formula transcriptions. Roughly 80% of entries match their book; most of the rest are labelled adaptations. Raw findings, page references and the book-first rule extractions are kept privately (not in Git).

## Candidate code defects (each needs a book-page re-read and before/after numbers before any change)
- **acbff_5c_day, 5c_perp, 5c1_day, 5c1_perp** (acbff.rs, run_rev calls): buy and short sides appear inverted versus Tables 36.1-36.3. All four are withheld, so nothing wrong is published.
- **Ruggiero runaway-gap 5-day filter** (rgc): the book's Table 4.4 short result equals Table 4.3, suggesting the filter was long-only; the catalogue applies it to both sides.
- **ss_turtle_soup**: the book's day-1/day-2 re-entry rule (rule 6, p.26) is not implemented or documented.
- **kpss holidays()**: Memorial Day 1970 (a Saturday) is moved to Friday 29 May, but the Dow traded that day; affects about ten holiday strategies for 1970 only.
- **spmt_dt_gapup**: buys all unfilled pieces (full size) where the book says a small position (held, unverified).
- **MetaStock engine**: closes a position only through its own Close formula and ignores an opposite entry signal (158 strategies); MetaStock's Formula Primer says its tester also exits on an opposite entry. The exact MetaStock behaviour needs a licensed copy to confirm. Operator precedence (251 strategies) is also unconfirmed.
- **mstt_price_momentum**: the print gives Enter Long and Enter Short the identical formula; transcribed exactly with no note.

## Gate and labelling problems
- **Same-day withhold rule** (accuracy gate, 12 Oct): about 190 strategies are withheld because daily bars cannot show which price came first. For plain open-to-close trades with no stop or limit this is wrong; it should apply only where a stop or limit inside the bar decides the outcome.
- **"Matches the book" is a loose statistical band** (trade count within about 2 or 25%, win rate within 8 points, sign of the average only). Where audits compared ours with the book, ours was at least as good every time (S&P Market Timing: win rate +5.5 points, average trade x1.32; Stridsman Gold Digger trades +19-51%). Causes include the book's costs, unadjusted ETF prices and different instruments; not all of the gap is explained.
- Several held reasons and page citations are wrong (Katz, Long-Term Secrets, Collins); recorded privately.

## Not covered
Stridsman: about 40% of pages unread (scan, no text layer). Million Dollar Challenge: 28 image-only pages. MetaStock add-on pack: binary files unreadable here. Author's CD code for the Katz book is not in the PDF.
