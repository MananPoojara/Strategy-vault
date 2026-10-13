import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  publicDir: "public",
  server: {
    port: 5173,
    proxy: Object.fromEntries(
      [
        "/api",
        "/data",
        "/login.html",
        "/login.js",
        "/login.css",
        "/admin.html",
        "/admin.js",
      ].map((path) => [
        path,
        {
          target: process.env.VAULT_DEV_PROXY || "http://127.0.0.1:8180",
          changeOrigin: false,
        },
      ]),
    ),
  },
  test: {
    environment: "jsdom",
    maxWorkers: 1,
    coverage: {
      provider: "v8",
      include: [
        "src/state/store.ts",
        "src/services/api.ts",
        "src/services/library-scope.ts",
        "src/services/instrument-controls.ts",
        "src/services/strategy-indicators.ts",
        "src/app/routes.ts",
        "src/components/ResearchMarkup.tsx",
      ],
      thresholds: { lines: 80, functions: 80, branches: 80, statements: 80 },
    },
  },
  build: { outDir: "dist", emptyOutDir: true, chunkSizeWarningLimit: 750 },
});
