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
        "src/services/library-market.ts",
        "src/services/author-order.ts",
        "src/services/library-backtest-context.ts",
        "src/services/instrument-controls.ts",
        "src/services/strategy-indicators.ts",
        "src/app/routes.ts",
        "src/components/ResearchMarkup.tsx",
        "src/features/lab/url-state.ts",
        "src/features/lab/labels.ts",
        "src/features/lab/client.ts",
        "src/features/lab/format.ts",
        "src/features/lab/spec-model.ts",
        "src/features/lab/charts.ts",
        "src/features/lab/bars.ts",
        "src/features/lab/catalogue.ts",
        "src/features/lab/instruments.ts",
        "src/features/lab/lab-state.ts",
      ],
      thresholds: { lines: 80, functions: 80, branches: 80, statements: 80 },
    },
  },
  build: { outDir: "dist", emptyOutDir: true, chunkSizeWarningLimit: 750 },
});
