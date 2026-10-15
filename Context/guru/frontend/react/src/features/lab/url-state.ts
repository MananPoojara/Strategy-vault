import { resolveRoute } from "../../app/routes";
import type { ChartRange, Market } from "./types";

export interface LabQuery { region: Market; tk: string; tf: string; range: ChartRange; customize: string; strategy: string; published: string }
const DEFAULTS: LabQuery = { region: "in", tk: "", tf: "D", range: "3Y", customize: "", strategy: "", published: "" };

export function readLabQuery(hash = location.hash): LabQuery {
  const q = resolveRoute(hash).query;
  const region = q.get("region");
  const range = q.get("range");
  return {
    region: region === "us" || region === "gl" ? region : "in",
    tk: q.get("tk") ?? "",
    tf: q.get("tf") || "D",
    range: range === "1Y" || range === "All" ? range : "3Y",
    customize: q.get("customize") ?? "",
    strategy: q.get("strategy") ?? "",
    published: q.get("published") ?? "",
  };
}

/** Defaults and empty values are omitted; other keys in the current query are kept. */
export function labHash(path: string, patch: Partial<LabQuery>, hash = location.hash): string {
  const q = new URLSearchParams(resolveRoute(hash).query);
  for (const [key, value] of Object.entries(patch)) {
    if (value === "" || value === undefined || value === DEFAULTS[key as keyof LabQuery]) q.delete(key);
    else q.set(key, String(value));
  }
  const search = q.toString();
  return `#${path}${search ? `?${search}` : ""}`;
}

/** location.replace fires hashchange (so App saves lastView) without adding a history entry. */
export function writeLabQuery(patch: Partial<LabQuery>, path = resolveRoute(location.hash).path) {
  const next = labHash(path, patch);
  if (next !== location.hash) location.replace(next);
}
