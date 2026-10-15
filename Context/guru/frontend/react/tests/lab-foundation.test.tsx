import { afterEach, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { customPages } from "../src/app/custom-pages";
import { AppHeader } from "../src/components/AppHeader";
import { labHash, readLabQuery, writeLabQuery } from "../src/features/lab/url-state";
import { unavailableText, variantLabel, LAB_LABEL, PROVISIONAL_NOTE } from "../src/features/lab/labels";
import type { ViewSnapshot } from "../src/state/store";

afterEach(() => history.replaceState(null, "", "/"));

it("registers both lab pages in navigation", () => {
  expect(customPages.map((p) => [p.path, p.label, p.showInNavigation])).toEqual([
    ["/indicators", "Indicators", true],
    ["/yt-strategies", "YT Strategies", true],
  ]);
});

it("shows one YT Strategies link, no 'soon' placeholder, and marks the open custom page", () => {
  history.replaceState(null, "", "/#/indicators?region=us");
  const snapshot = { route: { name: "home", path: "/", query: new URLSearchParams() }, status: "" } as unknown as ViewSnapshot;
  const html = renderToStaticMarkup(<AppHeader account={null} snapshot={snapshot} />);
  expect(html.match(/YT Strategies/g)).toHaveLength(1);
  expect(html).not.toContain("soon");
  expect(html).toContain('<a href="#/indicators" class="on">Indicators</a>');
  expect(html).not.toMatch(/data-r="home" class="on"/);
});

it("reads the page query with defaults and writes it without losing other keys", () => {
  expect(readLabQuery("#/indicators")).toEqual({ region: "in", tk: "", tf: "D", range: "3Y", customize: "", strategy: "", published: "" });
  expect(readLabQuery("#/indicators?region=gl&tk=CRYPTO%3ABTCUSD&tf=W&range=All").region).toBe("gl");
  expect(readLabQuery("#/indicators?region=xx").region).toBe("in");
  expect(labHash("/indicators", { tk: "NIFTY 50", customize: "" }, "#/indicators?region=us&customize=rsi2_4"))
    .toBe("#/indicators?region=us&tk=NIFTY+50");
  history.replaceState(null, "", "/#/indicators?region=us");
  writeLabQuery({ tf: "M" });
  expect(location.hash).toBe("#/indicators?region=us&tf=M");
});

it("keeps server labels verbatim and adds the unavailable prefix once", () => {
  expect(LAB_LABEL).toBe("User-defined indicator strategy · simulated · not a book method");
  expect(PROVISIONAL_NOTE).toBe("Provisional · TradingView export · internal research only, not publishable to YT Strategies");
  expect(variantLabel("Connors", "RSI 2")).toBe("Customized from Connors · RSI 2 · user variant · simulated");
  expect(unavailableText("the method is not admitted")).toBe("Customize not available — the method is not admitted");
  expect(unavailableText("Customize not available — x")).toBe("Customize not available — x");
});
