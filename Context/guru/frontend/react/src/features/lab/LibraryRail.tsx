import { useEffect, useState } from "react";
import { availability, groupByCategory, searchIndicators } from "./catalogue";
import type { Catalogue, Indicator, Market, Timeframe } from "./types";
import "./library.css";

const narrow = () => typeof window !== "undefined" && !!window.matchMedia?.("(max-width: 900px)").matches;

export function LibraryRail({ catalogue, market, hasVolume, tf, onAdd, onRead }: {
  catalogue: Catalogue; market: Market; hasVolume: boolean; tf: Timeframe; onAdd(ind: Indicator): void; onRead(ind: Indicator): void;
}) {
  const [query, setQuery] = useState("");
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
  const body = (
    <>
      <input type="search" value={query} onChange={(e) => setQuery(e.target.value)}
        placeholder="Search name, id, TradingView or AmiBroker name" aria-label="Search indicators" />
      {groups.length === 0 && <div className="empty">No indicator matches "{query}".</div>}
      {groups.map(([category, list]) => (
        <details open key={category}>
          <summary>{category} <small>{list.length}</small></summary>
          <ul>
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
        </details>
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
