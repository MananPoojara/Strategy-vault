import { useEffect, useState } from "react";
import { availability, groupByCategory, searchIndicators } from "./catalogue";
import type { Catalogue, Indicator, Market, Timeframe } from "./types";
import "./library.css";

const narrow = () => typeof window !== "undefined" && !!window.matchMedia?.("(max-width: 900px)").matches;

export function LibraryRail({ catalogue, market, hasVolume, tf, onAdd, onRead }: {
  catalogue: Catalogue; market: Market; hasVolume: boolean; tf: Timeframe; onAdd(ind: Indicator): void; onRead(ind: Indicator): void;
}) {
  const [query, setQuery] = useState("");
  const [manual, setManual] = useState<Record<string, boolean>>({}); // the user's own open/closed choices, kept across searches
  const [searchShut, setSearchShut] = useState<Set<string>>(new Set()); // groups closed by hand while a search is showing
  const [mobile, setMobile] = useState(narrow);
  useEffect(() => {
    const mq = window.matchMedia?.("(max-width: 900px)");
    if (!mq) return;
    const on = () => setMobile(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);
  const count = `${catalogue.indicators.length} indicators`;
  const groups = groupByCategory(searchIndicators(catalogue.indicators, query));
  const searching = query.trim() !== "";
  const isOpen = (category: string) => (searching ? !searchShut.has(category) : !!manual[category]);
  const toggle = (category: string) => searching
    ? setSearchShut((s) => { const n = new Set(s); if (!n.delete(category)) n.add(category); return n; })
    : setManual((m) => ({ ...m, [category]: !m[category] }));
  const body = (
    <>
      <input type="search" value={query} onChange={(e) => { setQuery(e.target.value); setSearchShut(new Set()); }}
        placeholder="Search name, id, TradingView or AmiBroker name" aria-label="Search indicators" />
      {groups.length === 0 && <div className="empty">No indicator matches "{query}".</div>}
      {groups.map(([category, list]) => (
        <div className="lab-group" key={category}>
          <button type="button" className="lab-group-h" aria-expanded={isOpen(category)} onClick={() => toggle(category)}>
            {category} <small>{list.length}</small>
          </button>
          <ul hidden={!isOpen(category)}>
            {list.map((ind) => {
              const { ok, reason } = availability(ind, { market, hasVolume, tf });
              return (
                <li key={ind.id}>
                  <button type="button" data-lab-item={ind.id} disabled={!ok} title={reason} onClick={() => onAdd(ind)}>
                    {ind.name} <span className="tag">{ok ? ind.placement : "needs data"}</span>
                    {!ok && <small>{reason}</small>}
                  </button>{" "}
                  <button type="button" className="dbtn ghost sm" aria-label={`About ${ind.name}`} onClick={() => onRead(ind)}>i</button>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </>
  );
  return (
    <aside className="box lab-library" data-lab-library>
      {mobile
        ? <details><summary>Library <small>{count}</small></summary>{body}</details>
        : <><h2 className="box-h">Library <small>{count}</small></h2>{body}</>}
    </aside>
  );
}
