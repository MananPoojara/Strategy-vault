export type RouteName =
  | "home"
  | "author"
  | "strategies"
  | "strategy"
  | "optimization"
  | "reading"
  | "saved"
  | "not-found";
export interface Route {
  name: RouteName;
  path: string;
  id?: string;
  query: URLSearchParams;
}
/** Preserve existing #/ links and filters so saved and shared links keep working. */
export function resolveRoute(hash: string): Route {
  const [path = "/", search = ""] = hash.replace(/^#/, "").split("?");
  const parts = path.split("/").filter(Boolean);
  const query = new URLSearchParams(search);
  const route: Route = { name: "not-found", path: path || "/", query };
  if (!parts.length) return { ...route, name: "home" };
  if (parts.length === 1 && parts[0] === "strategies")
    return { ...route, name: "strategies" };
  if (parts.length === 1 && parts[0] === "saved")
    return { ...route, name: "saved" };
  const names: Record<string, RouteName> = {
    guru: "author",
    s: "strategy",
    o: "optimization",
    source: "reading",
  };
  if (parts.length === 2 && names[parts[0]])
    try {
      return {
        ...route,
        name: names[parts[0]],
        id: decodeURIComponent(parts[1]),
      };
    } catch {
      return route;
    }
  return route;
}
