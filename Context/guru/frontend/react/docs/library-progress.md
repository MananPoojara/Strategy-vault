# Library counts and book progress

The home page reports three library-wide counts. Instrument and market filters affect browsing and result links, not these totals.

- **Total Authors:** explicitly identified people in `src/services/author-identities.ts`, deduplicated across curated credits and reviewed source bylines. This is the identified count so far. Do not count folders, organizations, publisher names or unresolved source credits as people. Retain their reading records.
- **Total Strategies found:** distinct `book:family` entries in the engine catalogue, including entries awaiting review. Parameter variants within a book count once. The same idea in two books remains two attributed entries. This count does not establish that every rule is complete, implemented or verified.
- **Backtested Strategies:** distinct methods with an admitted, finite measured summary containing closed trades, recorded in the current availability snapshot. Union across supported instruments and markets; never sum the same method's results across instruments. This records available simulated results, not original book-performance reproduction.

Author cards, the author dialog and author pages show Books, Strategies and Backtested. Book counts preserve indexed titles/editions, including separately labelled courses and articles; unresolved bibliographic records are not silently merged into published books. Only explicit reviewed source identities can merge a source row with a catalogue title. All original associations and source files must remain represented in the private inventory.

Book statuses:

- **Backtested:** at least one method has an available computed result. Other methods in the same book may still need work.
- **In progress:** queued, unreviewed or conversion incomplete. Does not claim an agent is processing every title. `source_review_required` and other pending admission reviews stay here, even if a legacy API marks them `not_testable`.
- **Not Backtested:** registered methods have explicit present blockers, such as missing input data or incomplete mechanical rules. Show the reason. This does not declare permanent impossibility. Topic keywords alone never establish non-convertibility.

## Availability and Excel refresh

`data/library-progress.json` is private runtime metadata, mounted with the existing frontend datasets. Never commit the generated snapshot, source inventory, workbook or evidence bodies. `library-progress.ts` verifies its schema, exact catalogue variant IDs and rule/admission signatures, method/book identity, evidence, live status and admitted instrument types before counting results. Missing or stale snapshots show a checking notice, not inferred completion.

A snapshot contains `schema_version: 1`, `as_of`, `catalogue_variant_ids`, `catalogue_rule_signatures`, `measured_backtests` and reviewed `approved_source_book_links`. Each measured row has `variant_id`, `book_id`, `method_id` (`book:family`), `author_id`, `market`, `instrument`, `result_status: "available"` and evidence locating the actual summary and readiness bodies. Eligibility must be checked with the production library discovery rules. Snapshot collection is a bounded maintenance job; never run a whole-corpus backtest at startup.

After each strategy admission or source correction, refresh the snapshot against the deployed catalogue and supported measured contexts, regenerate the private author/book workbook, recalculate its formulas, and verify coverage and yellow-row eligibility before replacing the previous version. Yellow identifies only books with available site backtests; source rows require an explicit reviewed identity link to the corresponding completed book. Preserve the inventory date and distinguish indexed source units from verified distinct published books.

The private working directory and refresh commands are recorded in the conversion continuation checkpoint. Refresh remains required when the conversion goal resumes; a paused goal does not imply background conversion.
