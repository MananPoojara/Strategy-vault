import { useEffect, useRef, useState } from "react";
import type { HomeModel } from "../state/store";
import { matchingSearchAuthors, type SearchAuthor } from "../services/library-search";
import {
  activeFilters, BOOK_STATUS_LABELS, CLEAR_ALL, SCOPE_LABELS, STATUS_LABELS, TOPIC_LABELS, TYPE_LABELS,
} from "../services/author-filters";

type Item = [value: string, label: string] | { heading: string };
const patch = (next: Record<string, string>) =>
  window.VaultRuntime.patch({ ...next, library_limit: "24", library_page: "1", source_page: "1" });
const entries = (labels: Record<string, string>): Item[] => Object.entries(labels);

/** One pill of the filter bar: the same <details class="dd"> markup and CSS as the strategies page. */
function Dropdown({ name, items, current, counts, all, shown, onPick }: {
  name: string;
  items: Item[];
  current: string;
  counts?: Record<string, number>;
  /** Label of the reset option; a pill without it always has a value (Browse, Market, Sort). */
  all?: string;
  /** Text after the name; defaults to the chosen option. */
  shown?: string;
  onPick: (value: string) => void;
}) {
  const isSet = all !== undefined && current !== "";
  const label = shown ?? (items.find((item) => Array.isArray(item) && item[0] === current) as [string, string] | undefined)?.[1];
  const option = (value: string, text: string) => {
    const count = counts?.[value], active = current === value;
    return (
      <li key={value}>
        <button type="button" className={active ? "active" : ""} aria-pressed={active}
          disabled={!active && count === 0} onClick={() => onPick(value)}>
          <span>{text}</span>
          {count != null && <span className="badge">{count.toLocaleString("en-IN")}</span>}
        </button>
      </li>
    );
  };
  return (
    <details className={`dd ${isSet ? "set" : ""}`}>
      <summary className="dd-btn">
        <span className="dd-l">{name}</span>
        {(isSet || all === undefined) && label ? <b>{label}</b> : null}
        <i className="chev" aria-hidden="true" />
      </summary>
      <ul className="menu" aria-label={name}>
        {all !== undefined && option("", all)}
        {items.map((item) => Array.isArray(item)
          ? option(item[0], item[1])
          : <li key={"h:" + item.heading} className="menu-h" aria-hidden="true">{item.heading}</li>)}
      </ul>
    </details>
  );
}

function FilterSearch({ query }: { query: string }) {
  const [value, setValue] = useState(query);
  const timer = useRef(0);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const send = (next: string) => { window.clearTimeout(timer.current); if (next !== query) patch({ library_q: next }); };
  return (
    <label className="dsearch">
      <span className="sr-only">Search authors, books and strategies</span>
      {/* The page remounts on each change; the runtime restores focus and caret by this id. */}
      <input id="library-filter-q" type="search" value={value} autoComplete="off"
        placeholder="Search author, book, strategy…"
        onChange={(event) => {
          const next = event.target.value;
          setValue(next);
          window.clearTimeout(timer.current);
          timer.current = window.setTimeout(() => send(next), 300);
        }}
        onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); send(value); } }} />
    </label>
  );
}

const AUTHOR_SORTS: Item[] = [["default", "Backtests first"], ["strategies", "Most strategies"], ["books", "Most books"], ["name", "A–Z"]];
const BOOK_SORTS: Item[] = [["default", "Catalogue order"], ["name", "Title A–Z"], ["name-desc", "Title Z–A"], ["author", "Author A–Z"], ["newest", "Newest edition"]];

export function LibraryControls({ model, state }: { model: HomeModel; state: Record<string, string> }) {
  const books = model.view === "books", counts = model.filterCounts || {};
  const region = state.region === "us" ? "us" : "in";
  const choices = model.marketChoices || { in: region === "in" ? model.indices : [], us: region === "us" ? model.indices : [] };
  const baskets = model.baskets.filter((basket) => basket.symbols.length);
  const instrument = (list: { ticker: string; name: string }[]): Item[] => list.map((item) => ["i:" + item.ticker, item.name]);
  const basketItems: Item[] = baskets.length ? [{ heading: `${region === "us" ? "US" : "India"} stock baskets` }, ...baskets.map((basket): Item => ["b:" + basket.id, /stock/i.test(basket.name) ? basket.name : `${basket.name} · stocks`])] : [];
  const marketItems: Item[] = [
    ["all", "All markets"],
    { heading: "India" }, ...instrument(choices.in), ...(region === "in" ? basketItems : []),
    ...(choices.us.length ? [{ heading: "US" }, ...instrument(choices.us), ...(region === "us" ? instrument(model.etfs || []) : []), ...(region === "us" ? basketItems : [])] : []),
  ];
  const currentInstrument = state.basket ? "b:" + state.basket : "i:" + state.tk;
  const instrumentName = (marketItems.find((item) => Array.isArray(item) && item[0] === currentInstrument) as [string, string] | undefined)?.[1] || model.symbol;
  const pickMarket = (value: string) => {
    if (value === "all") return patch({ library_market: "all" });
    const ticker = value.slice(2), market = value.startsWith("b:") ? region : choices.in.some((item) => item.ticker === ticker) ? "in" : "us";
    const basket = value.startsWith("b:") ? baskets.find((item) => item.id === ticker) : undefined;
    patch({ library_market: market, region: market, basket: basket?.id || "", tk: basket ? basket.symbols[0] : ticker, from: "", to: "" });
  };
  const chips = activeFilters(state, model.view);
  return (
    <section className="fbar library-fbar" aria-label="Library filters">
      <div className="fbar-in">
        <Dropdown name="Browse" current={books ? "books" : "authors"} items={[["authors", "Authors"], ["books", "Books & reading"]]}
          onPick={(value) => patch({ library_view: value, library_sort: "default" })} />
        {!books && <Dropdown name="Market" items={marketItems} current={model.market === "all" ? "all" : currentInstrument}
          shown={model.market === "all" ? `All · ${instrumentName}` : instrumentName} onPick={pickMarket} />}
        {!books && state.basket && <Dropdown name="Stock" items={model.symbols.map((item): Item => [item.ticker, item.name])} current={state.tk}
          onPick={(value) => patch({ tk: value, from: "", to: "" })} />}
        {books
          ? <Dropdown name="Status" all="All statuses" items={entries(BOOK_STATUS_LABELS)} current={state.book_progress || ""} counts={counts.book_progress}
            onPick={(value) => patch({ book_progress: value })} />
          : <Dropdown name="Status" all="All authors" items={entries(STATUS_LABELS)} current={state.author_status || ""} counts={counts.author_status}
            onPick={(value) => patch({ author_status: value })} />}
        <Dropdown name="Topic" all="All topics" items={entries(TOPIC_LABELS)} current={model.category} counts={counts.book_category}
          onPick={(value) => patch({ book_category: value })} />
        {!books && <Dropdown name="Type" all="People and collections" items={entries(TYPE_LABELS)} current={state.author_type || ""} counts={counts.author_type}
          onPick={(value) => patch({ author_type: value })} />}
        <Dropdown name="Sort" items={books ? BOOK_SORTS : AUTHOR_SORTS} current={state.library_sort || "default"}
          onPick={(value) => patch({ library_sort: value })} />
        <Dropdown name="Library" items={entries(SCOPE_LABELS)} current={state.library_scope || "trading"} counts={counts.library_scope}
          onPick={(value) => patch({ library_scope: value })} />
        <FilterSearch query={model.query} />
      </div>
      {chips.length > 0 && (
        <div className="chips">
          {chips.map((chip) => (
            <span className="chip" key={chip.key}>
              {chip.label}
              <button type="button" aria-label={`Remove ${chip.label}`} onClick={() => patch(chip.clear)}>×</button>
            </span>
          ))}
          <button type="button" className="link" onClick={() => patch(CLEAR_ALL)}>Clear all</button>
        </div>
      )}
    </section>
  );
}
function SearchAvatar({author}: {author: SearchAuthor}) {
  const [failed, setFailed] = useState(false);
  return <span className="search-avatar" aria-hidden="true">
    {author.photo && !failed
      ? <img src={author.photo} alt="" onError={() => setFailed(true)} />
      : <span>{author.initials}</span>}
  </span>;
}

export function LibrarySearch({ query, authors = [] }: { query: string; authors?: SearchAuthor[] }) {
  const [value, setValue] = useState(query);
  const [focused, setFocused] = useState(false);
  const [active, setActive] = useState(-1);
  useEffect(() => { setValue(query); setActive(-1); }, [query]);
  const matches = matchingSearchAuthors(authors, value);
  const expanded = focused && value.trim().length > 0;
  const selected = expanded ? matches[active] : undefined;
  return (
    <form
      className="library-search"
      id="library-search-form"
      role="search"
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setFocused(false);
      }}
      onSubmit={(event) => {
        event.preventDefault();
        event.stopPropagation();
        setFocused(false);
        setActive(-1);
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
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={expanded}
        aria-controls={expanded ? "library-author-suggestions" : undefined}
        aria-activedescendant={selected ? `search-author-${selected.id}` : undefined}
        onFocus={() => setFocused(value.trim() !== query.trim())}
        onKeyDown={(event) => {
          if (event.key === "Escape" || event.key === "Tab") {
            if (event.key === "Escape" && expanded) {
              event.preventDefault();
              event.stopPropagation();
            }
            setFocused(false);
            setActive(-1);
            return;
          }
          if ((event.key === "ArrowDown" || event.key === "ArrowUp") && matches.length) {
            event.preventDefault();
            setFocused(true);
            setActive(index => event.key === "ArrowDown"
              ? (index + 1) % matches.length
              : (index <= 0 ? matches.length - 1 : index - 1));
          }
          if (event.key === "Enter" && selected && !event.nativeEvent.isComposing) {
            event.preventDefault();
            event.stopPropagation();
            setFocused(false);
            window.location.assign(selected.href);
          }
        }}
        onChange={(event) => {
          event.stopPropagation();
          setValue(event.target.value);
          setFocused(true);
          setActive(-1);
        }}
      />
      <button type="submit">
        Search <span aria-hidden="true">↗</span>
      </button>
      {expanded && <div className="search-suggestions">
        <div className="search-suggestions-heading">Authors</div>
        <div id="library-author-suggestions" role="listbox" aria-label="Matching authors">
          {matches.map((author, index) => <a
            key={author.id}
            id={`search-author-${author.id}`}
            className="search-author"
            role="option"
            aria-selected={active === index}
            href={author.href}
            tabIndex={-1}
            onMouseEnter={() => setActive(index)}
            onMouseDown={event => event.preventDefault()}
            onClick={() => setFocused(false)}
          >
            <SearchAvatar author={author} />
            <span className="search-author-copy"><strong>{author.name}</strong><small>Open author library</small></span>
            <span className="search-author-arrow" aria-hidden="true">↗</span>
          </a>)}
        </div>
        {!matches.length && <p className="search-no-authors">No matching authors</p>}
        <p className="search-suggestions-hint">{selected ? "Enter to open author · Esc to close" : "Press Enter to search all books and authors"}</p>
      </div>}
    </form>
  );
}
