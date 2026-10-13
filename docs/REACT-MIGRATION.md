# React migration

The production entry is `Context/guru/frontend/react/index.html`; `src/main.tsx` mounts one React application. The original look and themes are retained in `src/styles.css`. Old full-variant1 shared links redirect to the React application, preserving the hash/query.

Native TSX components implement the header/account controls, home/library search/filter/card/pagination UI, standalone source reading pages and Saved. The typed API service keeps existing cookie authentication. A typed route resolver and `custom-pages.ts` let another developer add a page within the shared layout. The immutable view store rejects stale commits.

Source/rule/author/backtest/optimization presenters retained in `src/compat/runtime.js` are not new trading calculations. Existing Rust rules and instrument policies remain unchanged. `ResearchMarkup` converts their presentation output into React elements, removes scripts and unknown inline handlers, and permits only reviewed author actions. New pages use TSX directly.

Chart and export libraries still draw/update their own widget DOM. The React result boundary remounts per completed result revision and cleans chart observers before replacing a view. This avoids React trying to update chart-managed children. The compatibility layer remains a maintenance debt; not every old renderer has been rewritten into small typed JSX components. It is documented rather than represented as a complete widget rewrite.

The repo has no embedded market datasets or original books. Gateway metadata/download inputs are mounted read-only, separate from the compiled React assets. Login/admin pages remain the established gateway-served screens; their authentication behaviour is preserved.

The default library is now trading-focused. General programming/plotting guides are supporting technical references, and titles whose market relevance is unknown are held in a separate pending-relevance view. The full source inventory is retained explicitly. Relevance screening uses declared titles/filenames and curated associations; it is not full-book review or rule certification. The headline counts the source groups in the selected relevance view, without calling metadata groups verified authors. The 71 curated trading collections and the complete 4,745-group inventory are separately identified in expandable coverage details. Financial programming titles (for example, algorithmic trading) can remain relevant without being claimed as complete mechanical strategies.
