import type { ViewSnapshot } from "../state/store";
import { LibraryControls, LibrarySearch } from "../components/LibraryControls";
import { AuthorCard } from "../components/AuthorCard";
import { BookCard } from "../components/BookCard";
import { Pagination } from "../components/Pagination";
import { DataCoverageNotice } from "../components/DataCoverageNotice";
import { LivingBook } from "../components/LivingBook";
import { shareReview as share, shareFiles as files } from "../services/source-review-data";
import { CLEAR_ALL } from "../services/author-filters";
const n = (v: number) => v.toLocaleString("en-IN");
export function HomePage({ snapshot }: { snapshot: ViewSnapshot }) {
  const model = snapshot.home;
  if (!model) return null;
  return (
    <>
      <section className="lhero">
        <div className="hero-art" aria-hidden="true">
          <LivingBook />
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
          ["Total Authors", share.authors],
          ["Total Books", share.books],
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
        Authors and Books cover every trading source in the research share, each checked from its first pages: authors are distinct credited people, and a copy, edition or translation counts once.
        The share also holds {share.articles} articles, {share.courseManuals} course manuals, {share.codePackages} system code packages, {share.magazinesAndNewsletters} magazines and newsletters, {share.videoCourses} video courses and {share.otherSources} other sources.
        {model.libraryTotals ? ` Strategies so far come from ${model.libraryTotals.books} of these books and ${model.libraryTotals.otherSources} other sources.` : ""}{" "}
        Total Strategies includes catalogued entries awaiting review or conversion, grouping parameter versions within each book.
        Backtested counts strategies with available computed results on a supported instrument,
        across India and US; it does not certify the original book study.
      </p>
      <section className="share-files" aria-labelledby="share-files-h">
        <h2 id="share-files-h" className="section-h">Research share files</h2>
        <dl className="lstrip share-files-strip">
          {([["Authors", files.authors], ["Book files", files.books], ["Articles", files.articles], ["Research papers", files.papers], ["Audio", files.audio], ["Video", files.video], ["Unique files", files.unique]] as const).map(([label, value]) => (
            <div key={label}><dt>{label}</dt><dd>{n(value)}</dd></div>
          ))}
        </dl>
        <p className="library-count-note">
          Every file in the research share, counted on {new Date(files.asOf).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })}: {n(files.found)} files found, {n(files.duplicates)} duplicate copies removed (same name and size in more than one folder), {n(files.unique)} unique files ({files.sizeTB} TB).
          The other {n(files.other)} files support the material above: {n(files.images)} images and {n(files.webPages)} web pages (mostly books and courses saved as web pages), {n(files.tradingCode)} trading software files, {n(files.data)} spreadsheets and data, {n(files.programs)} programs and installers, {n(files.archives)} archives, {n(files.text)} text notes, {n(files.blogs)} blog files and {n(files.noExtension + files.unidentified)} files of other types.
          Authors are the {n(files.authors)} distinct people named on these files (co-authors counted separately, spelling variants of one name merged); {n(files.companies)} companies and websites are not counted as authors.
          These are file counts; one book can be several files, so they differ from the reviewed book and author totals above.
        </p>
      </section>
      {!model.progressCurrent ? <p className="rnote" role="status">Backtest progress is being checked against the current catalogue. No completion is inferred from enabled code.</p> : null}
      <LibraryControls model={model} state={snapshot.state} />
      <DataCoverageNotice issues={model.summaryIssues} />
      {model.resultsPending ? <p className="rnote" role="status" data-results-pending>
        Backtest details for {model.symbol} are still calculating and fill in here when ready. The counts above are already final.
      </p> : null}
      {model.view === "authors" && (snapshot.state.library_sort || "default") === "default" ? <p className="library-backtest-note">
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
            {model.view === "books"
              ? model.totals && model.page.total !== model.totals.books
                ? `Showing ${model.page.total} of ${model.totals.books} indexed entries`
                : `${model.page.total} indexed entries`
              : model.listedPeople !== undefined
                ? model.totals && model.page.total !== model.totals.people + model.totals.collections
                  ? `Showing ${model.listedPeople} of ${model.totals.people} authors · ${model.page.total - model.listedPeople} of ${model.totals.collections} collections`
                  : `${model.listedPeople} authors · ${model.page.total - model.listedPeople} collections`
                : `${model.page.total} author and collection records`}
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
              onClick={() => window.VaultRuntime.patch({ ...CLEAR_ALL, library_page: "1" })}
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
