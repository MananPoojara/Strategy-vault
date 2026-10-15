import { expect, it } from "vitest";
import { route } from "../scripts/lab-dev-proxy.mjs"; // allowJs resolves it; checkJs is off

it("maps market engine paths, stubs the account and refuses everything else", () => {
  const env = { LAB_ENGINE_IN: "http://127.0.0.1:18090" };
  expect(route("GET", "/api/in/lab/run?spec=%7B%7D&ticker=NIFTY+50", env)).toEqual({ kind: "engine", url: "http://127.0.0.1:18090/api/lab/run?spec=%7B%7D&ticker=NIFTY+50" });
  expect(route("GET", "/api/app/me", { LAB_DEV_ROLE: "viewer" })).toMatchObject({ kind: "json", status: 200, body: { user: { role: "viewer" } } });
  expect(route("GET", "/api/app/me", {})).toMatchObject({ body: { user: { role: "researcher" } } });
  expect(route("PUT", "/api/app/settings", {})).toMatchObject({ status: 200 });
  expect(route("GET", "/api/app/lab/strategies", {})).toMatchObject({ status: 503 });
  expect(route("POST", "/api/in/lab/run", env)).toMatchObject({ status: 405 });
  expect(route("GET", "/api/gl/tickers", { LAB_ENGINE_GL: "http://x" })).toMatchObject({ status: 400 });
  expect(route("GET", "/api/gl/lab/catalogue", { LAB_ENGINE_GL: "http://x" })).toEqual({ kind: "engine", url: "http://x/api/lab/catalogue" });
  expect(route("GET", "/api/us/lab/catalogue", {})).toMatchObject({ status: 503 });
  expect(route("GET", "/api/xx/lab/catalogue", env)).toMatchObject({ status: 404 });
});
