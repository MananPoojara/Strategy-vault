import type { BookCard as Model } from "../state/store";
const labels: Record<string, string> = {
  "rule-based": "Rule-based",
  psychology: "Psychology",
  "money-management": "Money management",
};
export function BookCard({ book }: { book: Model }) {
  return (
    <article className="library-book" data-library-book={book.id}>
      <span className="library-book-kind">
        {book.kind === "catalogue"
          ? "Catalogue book entry"
          : "Reading source · identity review"}
      </span>
      <p className="library-book-author">{book.author}</p>
      <h3>
        <a href={book.href}>{book.title}</a>
      </h3>
      {book.scope === "technical" ? <div className="book-types"><span>Supporting technical reference</span></div> : book.scope === "pending" ? <p className="category-pending">Market relevance review pending</p> : book.categories.length ? (
        <div className="book-types">
          {book.categories.map((category) => (
            <span
              key={category}
              title="Topic classification; independent backtest verification is separate"
            >
              {labels[category] || category}
            </span>
          ))}
        </div>
      ) : (
        <p className="category-pending">Topic review pending</p>
      )}
      <p className="library-book-summary">{book.summary}</p>
      <div className="library-book-foot">
        <span>{book.year}</span>
        <a href={book.href}>
          {book.live && book.categories.includes("rule-based")
            ? "Open book & rules"
            : "Open reading page"}{" "}
          <span aria-hidden="true">↗</span>
        </a>
      </div>
    </article>
  );
}
