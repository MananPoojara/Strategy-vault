export interface ReadingModel {
  scopeNotice: string;
  id: string;
  title: string;
  authorName: string;
  authorHref: string;
  credit: string;
  year: string;
  categories: string[];
  summary: string;
  publisher: string | null;
  files: { id: string; filename: string; attachment: boolean }[];
  source: boolean;
  sourceFiles: string[];
  countBasis: string;
  review: null | {
    status: string;
    limits: string;
    outline: { label: string; detail: string }[];
    rules: {
      name: string;
      completeness: string;
      specified: string[];
      needed: string[];
    }[];
    evidence: { file: string; pages: string; finding: string }[];
  };
}
const labels: Record<string, string> = {
  "rule-based": "Rule-based",
  psychology: "Psychology",
  "money-management": "Money management",
};
export function ReadingPage({ model }: { model: ReadingModel }) {
  return (
    <>
      <nav className="crumbs">
        <a href="#/">Guru’s</a>
        <span>/</span>
        <a href={model.authorHref}>{model.authorName}</a>
        <span>/</span>
        <span>{model.title}</span>
      </nav>
      <section className="box reading-page" data-reading-page={model.id}>
        <div className="reading-book-cover" aria-hidden="true">
          <span>{model.credit}</span>
          <strong>{model.title}</strong>
          <small>{model.year}</small>
        </div>
        <div className="reading-book-copy">
          <span className="olabel">Reading library</span>
          <p className="rnote" data-reading-scope>
            {model.scopeNotice}
          </p>
          <h1>{model.title}</h1>
          <p className="reading-credit">
            {model.credit} · {model.year}
          </p>
          {model.categories.length ? (
            <div className="book-types">
              {model.categories.map((category) => (
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
          <p className="lede">{model.summary}</p>
          <div className="reading-status">
            <strong>Read the source</strong>
            <p>
              This page is for reading and source review. Topic classification
              does not establish a backtested trading system. No performance
              figures are attributed to this reading entry.
            </p>
          </div>
          {model.publisher && (
            <p>
              <a
                className="olink"
                href={model.publisher}
                target="_blank"
                rel="noopener noreferrer"
              >
                Publisher’s book record
              </a>
            </p>
          )}
          {model.files.length ? (
            <details className="source-pdfs">
              <summary>Read original files ({model.files.length})</summary>
              <ul>
                {model.files.map((file) => (
                  <li key={file.id}>
                    <a
                      className="olink"
                      href={`/api/app/books/files/${encodeURIComponent(file.id)}`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      {file.filename}
                    </a>
                    {file.attachment && (
                      <small> Download original document</small>
                    )}
                  </li>
                ))}
              </ul>
              <p className="muted">
                Files are served from the checked local reading cache.
                Filename-based catalogue associations may require bibliographic
                review; a readable copy does not certify authorship, rules or
                published performance.
              </p>
            </details>
          ) : (
            <p className="rnote">
              Data missing: a verified local reading copy is not available for
              this entry.
            </p>
          )}
        </div>
      </section>
      {model.review && (
        <section className="box" data-reading-review>
          <div className="box-h">
            Source content review <small>{model.review.status}</small>
          </div>
          <p className="rnote">
            Reviewed source sections are listed below. {model.review.limits}
          </p>
          {model.review.outline.length > 0 && (
            <figure className="reading-topic-map" data-reading-concept-map>
              <figcaption>
                Topics in the reviewed sections{" "}
                <small>Educational topic diagram · no backtest results</small>
              </figcaption>
              <div>
                {model.review.outline.map((topic, index) => (
                  <article key={index}>
                    <strong>{topic.label}</strong>
                    <p>{topic.detail}</p>
                  </article>
                ))}
              </div>
            </figure>
          )}
          {model.review.rules.length > 0 && (
            <>
              <h3>Rules and conversion assessment</h3>
              <div className="tlog">
                <table className="tbl">
                  <thead>
                    <tr>
                      <th>Method</th>
                      <th>What is specified</th>
                      <th>What is needed</th>
                    </tr>
                  </thead>
                  <tbody>
                    {model.review.rules.map((rule, index) => (
                      <tr key={index}>
                        <td>
                          <strong>{rule.name}</strong>
                          <p>{rule.completeness}</p>
                        </td>
                        <td>
                          {rule.specified.map((text, index) => (
                            <p key={index}>{text}</p>
                          ))}
                        </td>
                        <td>
                          {rule.needed.map((text, index) => (
                            <p key={index}>{text}</p>
                          ))}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
          {model.review.evidence.length > 0 && (
            <details>
              <summary>Reviewed pages and source evidence</summary>
              <ul>
                {model.review.evidence.map((evidence, index) => (
                  <li key={index}>
                    <strong>{evidence.file}</strong>
                    {evidence.pages ? ` · PDF ${evidence.pages}` : ""}
                    <p>{evidence.finding}</p>
                  </li>
                ))}
              </ul>
            </details>
          )}
        </section>
      )}
      {model.source && (
        <section className="box">
          <div className="box-h">Source record</div>
          <p>{model.countBasis}</p>
          <ul>
            {model.sourceFiles.map((filename, index) => (
              <li key={index}>{filename}</li>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}
