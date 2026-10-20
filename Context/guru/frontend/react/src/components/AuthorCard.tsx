import type { AuthorCard as Model } from "../state/store";
export function AuthorCard({
  author,
  index,
}: {
  author: Model;
  index: number;
}) {
  return (
    <a
      href={author.href}
      className="legend-card live"
      data-library-author={author.id}
      data-author-availability={author.status}
    >
      <div className="lc-top">
        <span className="no">
          {String(index + 1).padStart(2, "0")} /{" "}
          {!author.curated
            ? "Source credit · identity review"
            : author.families
              ? "Strategy research"
              : "Reading library"}
        </span>
        <span className="keyhole" aria-hidden="true" />
      </div>
      <div className="lc-body">
        <div className={`pframe ${author.photo ? "portrait" : "mono"}`}>
          {author.photo ? (
            <img src={author.photo} alt={author.name} loading="lazy" />
          ) : null}
          <span>{author.initials}</span>
        </div>
        <div>
          <h3>{author.name}</h3>
          <div className="lbook">{author.preview}</div>
        </div>
      </div>
      <p className="lc-tag">{author.tagline}</p>
      <button
        type="button"
        className="lc-more"
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          window.VaultRuntime.openAuthor(author.id, event);
        }}
      >
        About this author
      </button>
      <dl className="lc-stats">
        <div>
          <dt
            title={`${author.catalogue} catalogue titles and ${author.reading} additional indexed sources. Editions, courses and articles stay labelled; source identities remain under review.`}
          >
            Books
          </dt>
          <dd data-library-entry-count={author.id}>{author.entries}</dd>
        </div>
        <div>
          <dt>Strategies</dt>
          <dd>{author.families}</dd>
        </div>
        <div>
          <dt title={`Distinct strategies with available computed results across supported instruments; original book performance is not certified`}>
            Backtested
          </dt>
          <dd>{author.available}</dd>
        </div>
      </dl>
      {author.availableContext ? (
        <p className="lc-result-context" data-backtest-context={author.id}>
          <strong>{author.availableContext}</strong>
          {author.contextChanged ? <span>Opens this dataset; no results on the selected test data.</span> : null}
        </p>
      ) : null}
      <p className="lc-coverage availability-copy" title={author.statusText}>
        {author.statusText}
      </p>
      <div className="lc-foot">
        <span className={`lc-status status-${author.status}`}>
          {author.available ? "Backtested" : author.statusTitle}
        </span>
        <b className="lc-open">
          {author.openLabel || "Open the vault"} <span aria-hidden="true">↗</span>
        </b>
      </div>
    </a>
  );
}
