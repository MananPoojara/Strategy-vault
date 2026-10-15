#!/usr/bin/env node
// Dev only: stands in for the gateway so `npm run dev` can drive the lab pages against a locally started engine.
// No auth: /api/app/me is a fixed local account; saved strategies need the real auth service and answer 503.
import http from "node:http";
import { pathToFileURL } from "node:url";

const json = (status, body) => ({ kind: "json", status, body });

export function route(method, url, env = process.env) {
  const { pathname, search } = new URL(url, "http://local");
  if (pathname === "/api/app/me")
    return json(200, { user: { id: "dev", name: "Local dev", email: "dev@localhost", role: env.LAB_DEV_ROLE || "researcher" }, settings: { theme: "aqua" } });
  if (pathname === "/api/app/settings") return json(200, {});
  if (pathname.startsWith("/api/app/")) return json(503, { error: "Saved strategies need the auth service; this local dev proxy has none." });
  const match = /^\/api\/(in|us|gl)\/(.+)$/.exec(pathname);
  if (!match) return json(404, { error: "Unknown endpoint." });
  const [, market, path] = match;
  if (method !== "GET") return json(405, { error: "GET required." });
  if (market === "gl" && !path.startsWith("lab/")) return json(400, { error: "Invalid market or query." });
  const name = `LAB_ENGINE_${market.toUpperCase()}`;
  const base = env[name] || (market === "in" ? "http://127.0.0.1:18090" : "");
  if (!base) return json(503, { error: `No local ${market} engine is configured (${name}).` });
  return { kind: "engine", url: `${base}/api/${path}${search}` };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.env.LAB_DEV_PROXY_PORT || 8188);
  http.createServer(async (req, res) => {
    const target = route(req.method || "GET", req.url || "/");
    if (target.kind === "json") {
      res.writeHead(target.status, { "Content-Type": "application/json" });
      return res.end(JSON.stringify(target.body));
    }
    try {
      const upstream = await fetch(target.url);
      const headers = { "Content-Type": upstream.headers.get("content-type") || "application/json" };
      const retry = upstream.headers.get("retry-after");
      if (retry) headers["Retry-After"] = retry;
      res.writeHead(upstream.status, headers);
      res.end(Buffer.from(await upstream.arrayBuffer()));
    } catch {
      res.writeHead(502, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: "The requested service did not respond. Please try again." }));
    }
  }).listen(port, "127.0.0.1", () => console.log(`lab dev proxy on http://127.0.0.1:${port}`));
}
