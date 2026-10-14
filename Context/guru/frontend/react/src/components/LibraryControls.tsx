import { useEffect, useState } from "react";
import type { HomeModel } from "../state/store";
const categories = [
  ["", "All books"],
  ["rule-based", "Rule-based"],
  ["psychology", "Psychology"],
  ["money-management", "Money management"],
];
export function LibraryControls({
  model,
  state,
}: {
  model: HomeModel;
  state: Record<string, string>;
}) {
  const etfs = state.region === "us" ? model.etfs || [] : [];
  const patch = (next: Record<string, string>) =>
    window.VaultRuntime.patch({
      ...next,
      library_limit: "24",
      library_page: "1",
      source_page: "1",
    });
  return (
    <section
      className="library-controls library-controls-react"
      aria-label="Library filters"
    >
      <div className="library-controls-main">
        <label className="library-sort">
          Library scope{" "}
          <select
            aria-label="Library scope"
            value={state.library_scope || "trading"}
            onChange={(event) => {
              event.stopPropagation();
              patch({ library_scope: event.target.value });
            }}
          >
            <option value="trading">Trading library</option>
            <option value="technical">Supporting technical material</option>
            <option value="pending">Relevance review pending</option>
            <option value="all">Full source inventory</option>
          </select>
        </label>
        <div
          className="join library-mode"
          role="group"
          aria-label="Browse library"
        >
          {[
            ["authors", "Authors"],
            ["books", "Books & reading"],
          ].map(([key, label]) => (
            <button
              key={key}
              className={`join-item ${model.view === key ? "active" : ""}`}
              onClick={() =>
                patch({ library_view: key, library_sort: "default" })
              }
            >
              {label}
            </button>
          ))}
        </div>
        <label className="library-sort">
          Sort{" "}
          <select
            id="library-sort"
            value={state.library_sort || "default"}
            onChange={(event) => {
              event.stopPropagation();
              patch({ library_sort: event.target.value });
            }}
          >
            <option value="default">
              {model.view === "books" ? "Catalogue order" : "Backtests first"}
            </option>
            <option value="name">
              {model.view === "books" ? "Title" : "Author"} A–Z
            </option>
            <option value="name-desc">
              {model.view === "books" ? "Title" : "Author"} Z–A
            </option>
            {model.view === "books" ? (
              <>
                <option value="author">Author A–Z</option>
                <option value="newest">Newest edition</option>
              </>
            ) : (
              <>
                <option value="books">Most library entries</option>
                <option value="available">Available backtests</option>
              </>
            )}
          </select>
        </label>
      </div>
      <div className="library-controls-secondary">
        {" "}
        <div
          className="library-categories"
          role="group"
          aria-label="Book category"
        >
          {categories.map(([key, label]) => (
            <button
              key={key}
              className={`category-filter ${model.category === key ? "active" : ""}`}
              aria-pressed={model.category === key}
              onClick={() => patch({ book_category: key })}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="library-market">
          <div className="join sm" aria-label="Market">
            {[
              ["all", "All"],
              ["in", "India"],
              ["us", "US"],
            ].map(([key, label]) => (
              <button
                key={key}
                className={`join-item ${model.market === key ? "active" : ""}`}
                aria-pressed={model.market === key}
                title={key === "all" ? "Browse both markets; results use the selected test data." : undefined}
                disabled={key === "us" && !model.usAvailable}
                onClick={() =>
                  key === "all" ? patch({library_market: "all"}) : patch({
                    library_market: key,
                    region: key,
                    basket: "",
                    tk: key === "us" ? "SPX" : "NIFTY 50",
                    from: "",
                    to: "",
                  })
                }
              >
                {label}
              </button>
            ))}
          </div>
        </div>
        <label className="library-sort library-context">
          {model.market === "all" ? `Test data (${state.region === "us" ? "US" : "India"})` : "Test data"}{" "}
          <select
            id="library-instrument"
            aria-label="Backtest data"
            value={state.basket ? `b:${state.basket}` : `i:${state.tk}`}
            onChange={(event) => {
              event.stopPropagation();
              const value = event.target.value;
              if (value.startsWith("i:"))
                window.VaultRuntime.patch({
                  basket: "",
                  tk: value.slice(2),
                  from: "",
                  to: "",
                });
              else {
                const basket = model.baskets.find(
                  (item) => item.id === value.slice(2),
                );
                if (basket?.symbols.length)
                  window.VaultRuntime.patch({
                    basket: basket.id,
                    tk: basket.symbols[0],
                    from: "",
                    to: "",
                  });
              }
            }}
          >
            <optgroup label="Indices">
              {model.indices.map((index) => (
                <option key={index.ticker} value={`i:${index.ticker}`}>
                  {index.name}
                </option>
              ))}
            </optgroup>
            {etfs.length > 0 && (
              <optgroup label="ETFs">
                {etfs.map((etf) => (
                  <option key={etf.ticker} value={`i:${etf.ticker}`}>
                    {etf.name}
                  </option>
                ))}
              </optgroup>
            )}
            <optgroup label="Stock baskets — independent symbol">
              {model.baskets.map((basket) => (
                <option
                  key={basket.id}
                  value={`b:${basket.id}`}
                  disabled={!basket.symbols.length}
                >
                  {basket.name}
                </option>
              ))}
            </optgroup>
          </select>
        </label>
        {state.basket && (
          <label className="library-sort library-symbol">
            Symbol{" "}
            <select
              id="library-stock"
              aria-label="Backtest stock"
              value={state.tk}
              onChange={(event) => {
                event.stopPropagation();
                window.VaultRuntime.patch({
                  tk: event.target.value,
                  from: "",
                  to: "",
                });
              }}
            >
              {model.symbols.map((symbol) => (
                <option key={symbol.ticker} value={symbol.ticker}>
                  {symbol.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>
    </section>
  );
}
export function LibrarySearch({ query }: { query: string }) {
  const [value, setValue] = useState(query);
  useEffect(() => {
    if (value === query) return;
    const timer = setTimeout(
      () =>
        window.VaultRuntime.patch({
          library_q: value,
          library_limit: "24",
          library_page: "1",
          source_page: "1",
        }),
      250,
    );
    return () => clearTimeout(timer);
  }, [value, query]);
  return (
    <form
      className="library-search"
      id="library-search-form"
      role="search"
      onSubmit={(event) => {
        event.preventDefault();
        event.stopPropagation();
        window.VaultRuntime.patch({
          library_q: value.trim(),
          library_limit: "24",
          library_page: "1",
          source_page: "1",
        });
      }}
    >
      <svg viewBox="0 0 24 24" className="ic ic-line" aria-hidden="true">
        <circle cx="10.5" cy="10.5" r="7.5" />
        <path d="m16 16 5 5" />
      </svg>
      <label className="sr-only" htmlFor="library-query">
        Search books and authors
      </label>
      <input
        id="library-query"
        type="search"
        value={value}
        placeholder="Search a book, author or title…"
        autoComplete="off"
        onChange={(event) => {
          event.stopPropagation();
          setValue(event.target.value);
        }}
      />
      <button type="submit">
        Search <span aria-hidden="true">↗</span>
      </button>
    </form>
  );
}
