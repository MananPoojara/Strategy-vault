import { useEffect, useState } from "react";
import { availability, groupByCategory, searchIndicators, shortName } from "./catalogue";
import { myId } from "./formula";
import { ErrorNotice } from "./ui";
import type { Catalogue, Indicator, Market, SavedIndicator, Timeframe } from "./types";
import { POPULAR, useSimple } from "./simple";
import "./library.css";

const MINE = "My indicators", POPULAR_GROUP = "Popular";
/** "My indicators" group: the user's saved formulas; absent `onEdit` hides the group (no account storage). */
export interface MineProps { list: SavedIndicator[]; error?: unknown; canEdit: boolean; onAdd(row: SavedIndicator): void; onEdit(row: SavedIndicator | null): void }
const narrow = () => typeof window !== "undefined" && !!window.matchMedia?.("(max-width: 900px)").matches;

export function LibraryRail({ catalogue, market, hasVolume, tf, onAdd, onRead, mine }: {
  catalogue: Catalogue; market: Market; hasVolume: boolean; tf: Timeframe; onAdd(ind: Indicator): void; onRead(ind: Indicator): void; mine?: MineProps;
}) {
  const [query, setQuery] = useState("");
  const [manual, setManual] = useState<Record<string, boolean>>({}); // the user's own open/closed choices, kept across searches
  const [searchShut, setSearchShut] = useState<Set<string>>(new Set()); // groups closed by hand while a search is showing
  const [mobile, setMobile] = useState(narrow);
  const simple = useSimple();
  const [all, setAll] = useState(false); // Simple view: Popular only until the analyst asks for every category
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
  const myList = mine?.list.filter((r) => !searching || r.name.toLowerCase().includes(query.trim().toLowerCase())) ?? [];
  const isOpen = (category: string) => (searching ? !searchShut.has(category) : manual[category] ?? category === POPULAR_GROUP); // categories start closed (Popular open); a click shows every indicator in it
  const popular = simple && !all && !searching;
  const popularList = POPULAR.map((id) => catalogue.indicators.find((i) => i.id === id)).filter((i): i is Indicator => !!i);
  const toggle = (category: string) => searching
    ? setSearchShut((s) => { const n = new Set(s); if (!n.delete(category)) n.add(category); return n; })
    : setManual((m) => ({ ...m, [category]: !(m[category] ?? false) }));
  const body = (
    <>
      <input type="search" value={query} onChange={(e) => { setQuery(e.target.value); setSearchShut(new Set()); }}
        placeholder="Search name, id, TradingView or AmiBroker name" aria-label="Search indicators" />
      {mine && (
        <div className="lab-group lab-mine" data-lab-mine>
          <button type="button" className="lab-group-h" aria-expanded={isOpen(MINE)} onClick={() => toggle(MINE)}>
            {MINE} <small>{myList.length}</small>
          </button>
          <ul hidden={!isOpen(MINE)}>
            {mine.canEdit && <li><button type="button" className="dbtn sm lab-mine-new" onClick={() => mine.onEdit(null)}>+ New indicator</button></li>}
            {mine.error !== undefined && mine.error !== null && <li><ErrorNotice error={mine.error} market={market} /></li>}
            {myList.map((row) => (
              <li key={row.id}>
                <button type="button" data-lab-item={myId(row.id)} title={row.formula} onClick={() => mine.onAdd(row)}>
                  <b title={row.name}>{row.name}</b><small>Custom formula</small>
                </button>
                <button type="button" className="dbtn ghost sm lab-mine-edit" aria-label={`${mine.canEdit ? "Edit" : "View"} ${row.name}`} onClick={() => mine.onEdit(row)}>{mine.canEdit ? "Edit" : "View"}</button>
              </li>
            ))}
            {!myList.length && !mine.error && <li className="cap">{searching ? "No saved indicator matches." : "No saved indicators yet."}</li>}
          </ul>
        </div>
      )}
      {groups.length === 0 && <div className="empty">No indicator matches "{query}".</div>}
      {(popular ? [[POPULAR_GROUP, popularList] as [string, Indicator[]]] : groups).map(([category, list]) => (
        <div className="lab-group" key={category}>
          <button type="button" className="lab-group-h" aria-expanded={isOpen(category)} onClick={() => toggle(category)}>
            {category} <small>{list.length}</small>
          </button>
          <ul hidden={!isOpen(category)}>
            {list.map((ind) => {
              const { ok, reason } = availability(ind, { market, hasVolume, tf, maxForeign: catalogue.max_foreign });
              return (
                <li key={ind.id}>
                  <button type="button" data-lab-item={ind.id} disabled={!ok} title={ok ? ind.definition : reason} onClick={() => onAdd(ind)}>
                    <b title={ind.name}>{shortName(ind)}</b>
                    <small>{!ok || ind.needs.second_symbol ? reason : ind.definition}</small>
                  </button>
                  {!ok && <span className={`tag${ind.needs.volume && !hasVolume ? " warn" : ""}`}>{ind.needs.volume && !hasVolume ? "volume" : "needs data"}</span>}
                  <button type="button" className="dbtn ghost sm" aria-label={`About ${ind.name}`} onClick={() => onRead(ind)}>i</button>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
      {popular && <button type="button" className="dbtn sm lab-popular-more" data-lab-show-all onClick={() => setAll(true)}>Show all {catalogue.indicators.length} indicators by category</button>}
      {simple && all && !searching && <button type="button" className="dbtn sm ghost lab-popular-more" onClick={() => setAll(false)}>Show popular only</button>}
      <p className="lab-lib-foot">Click a name to add it to the chart · <span className="tag">needs data</span> listed, not available yet · <b>i</b> formula and sources</p>
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
