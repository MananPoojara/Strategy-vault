import type { ViewSnapshot } from "../state/store";
import { LibraryControls, LibrarySearch } from "../components/LibraryControls";
import { AuthorCard } from "../components/AuthorCard";
import { BookCard } from "../components/BookCard";
import { Pagination } from "../components/Pagination";
import { ResearchMarkup } from "../components/ResearchMarkup";
export function HomePage({ snapshot }: { snapshot: ViewSnapshot }) {
  const model = snapshot.home;
  if (!model) return null;
  return (
    <>
      <section className="lhero">
        <div className="guilloche" aria-hidden="true">
          <ResearchMarkup html={model.guilloche} />
        </div>
        <h1>Guru's</h1>
        <p className="lede">
          Trading books, their strategy rules, and research on India and US
          markets. Backtests appear only where the required data and calculation
          support are available.
        </p>
        <LibrarySearch query={model.query} />
        <div className="hero-cta">
          <a className="dbtn primary" href={model.strategiesLink}>
            Browse all strategies
          </a>
          <a
            className="dbtn"
            href="data/research-share-book-inventory.xlsx"
            download
          >
            Download book inventory
          </a>
          <a className="dbtn ghost" href={model.connorsLink}>
            Larry Connors
          </a>
        </div>
      </section>
      <dl className="lstrip library-summary">
        {[
          [
            model.scope === "trading"
              ? "Trading source groups"
              : "Source groups in this view",
            model.authors,
          ],
          [
            model.scope === "trading"
              ? "Trading-library entries"
              : "Entries in this view",
            model.entries,
          ],
          ["Implemented methods", model.implementedMethods],
          [`Backtest families on ${model.symbol}`, model.families],
          [model.basket ? "Basket symbols" : "Indices", model.symbolCount],
        ].map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd data-count={value}>{value}</dd>
          </div>
        ))}
      </dl>
      <details className="library-coverage-note">
        <summary>Library coverage & review status</summary>
        <p data-library-scope-notice>
          {model.scope === "trading"
            ? "Trading-related reading and curated collections. General programming guides are supporting material; unknown relevance remains pending. A title match does not establish a complete strategy."
            : model.scope === "technical"
              ? "Supporting technical references. These entries are not counted as trading strategies or money-management books."
              : model.scope === "pending"
                ? "Source titles awaiting market-relevance review. These entries are not counted as trading books."
                : "Complete source inventory, including technical and unreviewed material. Metadata groups are not verified distinct authors."}{" "}
          <span>
            {model.inventoryGroups} metadata groups and {model.inventoryEntries}{" "}
            entries in the full inventory.
          </span>
        </p>
        <dl className="coverage-totals">
          <div>
            <dt>Enabled methods</dt>
            <dd>{model.implementedMethods}</dd>
          </div>
          <div>
            <dt>Enabled variants</dt>
            <dd>{model.enabledVariants}</dd>
          </div>
          <div>
            <dt>Book / course entries with enabled methods</dt>
            <dd>{model.implementedBooks}</dd>
          </div>
          <div>
            <dt>Curated trading collections</dt>
            <dd>{model.curatedAuthors}</dd>
          </div>
          <div>
            <dt>Full inventory metadata groups</dt>
            <dd>{model.inventoryGroups}</dd>
          </div>
          <div>
            <dt>Full inventory entries</dt>
            <dd>{model.inventoryEntries}</dd>
          </div>
        </dl>
        <p>
          Implemented methods counts enabled book-specific families; it does not
          certify original rules or published studies. Source groups are
          recorded credits and collections, not a certified count of distinct
          authors. Screened titles are candidates; unreviewed relevance remains
          available in its own view.
        </p>
      </details>
      <LibraryControls model={model} state={snapshot.state} />
      <div id="library-results" aria-live="polite">
        <div className="library-result-count">
          <h2 className="section-h">
            {model.view === "books"
              ? "Books & reading sources"
              : model.query
                ? "Search results"
                : "Trading authors & source credits"}
          </h2>
          <span>
            {model.page.total}{" "}
            {model.view === "books"
              ? "indexed entries"
              : "authors / source-credit groups"}
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
