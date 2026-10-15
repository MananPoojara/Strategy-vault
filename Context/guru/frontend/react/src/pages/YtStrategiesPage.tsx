import { useEffect, useState } from "react";
import { strategies } from "../features/lab/client";
import { COPY } from "../features/lab/labels";
import type { PublishedStrategy } from "../features/lab/types";
import { ErrorNotice, LabErrorBoundary, Loading } from "../features/lab/ui";

type Load = { items: PublishedStrategy[] } | { error: unknown } | null;

function PublishedList() {
  const [load, setLoad] = useState<Load>(null);
  useEffect(() => {
    let live = true;
    strategies.published().then(
      (items) => live && setLoad({ items: [...items].sort((a, b) => b.publishedAt.localeCompare(a.publishedAt)) }),
      (error) => live && setLoad({ error }),
    );
    return () => { live = false; };
  }, []);

  if (!load) return <Loading text="Loading YT Strategies…" />;
  if ("error" in load) return <ErrorNotice error={load.error} market="in" />;
  if (!load.items.length) return <div className="empty"><b>{COPY.ytEmpty}</b></div>;
  return (
    <div style={{ display: "grid", gap: 14, gridTemplateColumns: "repeat(auto-fill, minmax(min(100%, 320px), 1fr))" }}>
      {load.items.map((s) => (
        <article key={s.snapshotId} className="box lab-card">
          <h2>{s.name}</h2>
          {s.origin && <span className="tag">Customized from {s.origin.book} · {s.origin.method}</span>}
          <p className="rnote">{s.ownerName ?? "Former member"} · {s.market === "in" ? "India" : "US"} · {s.instrument}</p>
          <p className="rnote">{s.spec.entry.all.length} entry / {s.spec.exit.any.length} exit · published {s.publishedAt.slice(0, 10)}</p>
          {/* region is always written: labHash would drop the default "in", and the lab falls back to the research region. */}
          <a className="dbtn sm" href={`#/indicators?${new URLSearchParams({ region: s.market, tk: s.instrument, published: String(s.snapshotId) })}`}>
            Open in lab
          </a>
        </article>
      ))}
    </div>
  );
}

export function YtStrategiesPage() {
  return (
    <div className="lab-page" data-lab-page="yt-strategies">
      <LabErrorBoundary>
        <h1 className="st">YT Strategies</h1>
        <p className="rnote">{COPY.ytIntro}</p>
        <PublishedList />
      </LabErrorBoundary>
    </div>
  );
}
