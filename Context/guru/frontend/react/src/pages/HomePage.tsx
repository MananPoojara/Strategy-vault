import type { ViewSnapshot } from "../state/store";
import { LibraryControls, LibrarySearch } from "../components/LibraryControls";
import { AuthorCard } from "../components/AuthorCard";
import { BookCard } from "../components/BookCard";
import { Pagination } from "../components/Pagination";
import { ResearchMarkup } from "../components/ResearchMarkup";
import { DataCoverageNotice } from "../components/DataCoverageNotice";
export function HomePage({ snapshot }: { snapshot: ViewSnapshot }) {
  const model = snapshot.home;
  if (!model) return null;
  return (
    <>
      <section className="lhero">
        <div className="hero-art" aria-hidden="true">
          <div className="guilloche"><ResearchMarkup html={model.guilloche} /></div>
        </div>
        <h1>Guru's</h1>
        <p className="lede">
          Trading books, their strategy rules, and research on India and US
          markets. Backtests appear only where the required data and calculation
          support are available.
        </p>
        <LibrarySearch query={model.query} authors={model.searchAuthors || []} />
        <div className="hero-cta">
          <a className="dbtn primary" href={model.strategiesLink}>
            Browse all strategies
          </a>
        </div>
      </section>
      <dl className="lstrip library-summary library-summary-strategies">
        {[
          ["Total Authors", model.libraryTotals?.authors],
          ["Total Books", model.libraryTotals?.books],
          ["Total Strategies", model.strategiesFound],
          ["Backtested Strategies", model.progressCurrent ? model.backtestedStrategies : "Checking"],
        ].filter(([, value]) => value !== undefined).map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd data-count={value}>{value}</dd>
          </div>
        ))}
      </dl>
      <p className="library-count-note">
        Authors are the distinct people credited on the sources; Books are distinct published books (a second copy or edition counts once).
        {model.libraryTotals ? ` Strategies also come from ${model.libraryTotals.otherSources} other sources: course manuals, articles, newsletters and vendor system code.` : ""}{" "}
        Total Strategies includes catalogued entries awaiting review or conversion, grouping parameter versions within each book.
        Backtested counts strategies with available computed results on a supported instrument,
        across India and US; it does not certify the original book study.
      </p>
      {!model.progressCurrent ? <p className="rnote" role="status">Backtest progress is being checked against the current catalogue. No completion is inferred from enabled code.</p> : null}
      <LibraryControls model={model} state={snapshot.state} />
      <DataCoverageNotice issues={model.summaryIssues} />
      {model.view === "authors" ? <p className="library-backtest-note">
        Authors with available backtests appear first. Open an author to see all indexed books and their review status.
      </p> : null}
      <div id="library-results" aria-live="polite">
        <div className="library-result-count">
          <h2 className="section-h">
            {model.view === "books"
              ? "Books & reading sources"
              : model.query
                ? "Search results"
                : "Authors & collections"}
          </h2>
          <span>
            {model.page.total}{" "}
            {model.view === "books"
              ? "indexed entries"
              : "author and collection records"}
            {model.query ? ` matching “${model.query}”` : ""}
          </span>
        </div>
        {model.page.total ? (
          model.view === "books" ? (
            <div className="library-book-grid">
              {model.bookCards.map((book) => (
                <BookCard key={book.id} book={book} />
              ))}
            </div>
          ) : (
            <div className="gallery">
              {model.authorCards.map((author, index) => (
                <AuthorCard
                  key={author.id}
                  author={author}
                  index={(model.page.page - 1) * 24 + index}
                />
              ))}
            </div>
          )
        ) : (
          <div className="empty library-empty">
            <b>No matching books or authors</b>Try another title, author or
            category.
            <button
              className="dbtn sm"
              onClick={() =>
                window.VaultRuntime.patch({
                  library_q: "",
                  book_category: "",
                  library_page: "1",
                })
              }
            >
              Clear search and filters
            </button>
          </div>
        )}
        <Pagination page={model.page} />
      </div>
    </>
  );
}
