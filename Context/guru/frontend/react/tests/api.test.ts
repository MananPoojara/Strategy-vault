import { afterEach, it, expect, vi } from "vitest";
import { apiJson } from "../src/services/api";
afterEach(() => vi.unstubAllGlobals());
it("uses same-origin session cookies and preserves JSON payloads", async () => {
  const fetch = vi
    .fn()
    .mockResolvedValue(new Response('{"status":"ready"}', { status: 200 }));
  vi.stubGlobal("fetch", fetch);
  expect(await apiJson("/api/app/me")).toEqual({ status: "ready" });
  expect(fetch.mock.calls[0][1].credentials).toBe("same-origin");
});
it("reports failed API requests without interpreting them as results", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(
        new Response('{"error":"Data missing"}', { status: 400 }),
      ),
  );
  await expect(apiJson("/api/in/run")).rejects.toThrow("Data missing");
});
it("rejects requests outside the application API and handles non-JSON errors", async () => {
  await expect(apiJson("https://outside.invalid")).rejects.toThrow(
    "same-origin",
  );
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("Unavailable", { status: 503 })),
  );
  await expect(apiJson("/api/app/me")).rejects.toThrow("503");
});
it("account, settings and sign-out calls use the existing authenticated endpoints", async () => {
  const { getAccount, saveSettings, signOut } = await import(
    "../src/services/api"
  );
  const fetch = vi
    .fn()
    .mockImplementation(() =>
      Promise.resolve(new Response("{}", { status: 200 })),
    );
  vi.stubGlobal("fetch", fetch);
  await getAccount();
  await saveSettings({ theme: "forest" });
  await signOut();
  expect(fetch.mock.calls.map((call) => call[0])).toEqual([
    "/api/app/me",
    "/api/app/settings",
    "/api/auth/sign-out",
  ]);
  expect(fetch.mock.calls[1][1].body).toBe('{"theme":"forest"}');
  expect(fetch.mock.calls[2][1].method).toBe("POST");
});
