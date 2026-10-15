import { it, expect } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ResearchMarkup } from "../src/components/ResearchMarkup";
it("renders research tables as React nodes, without scripts or executable attributes", () => {
  const html = renderToStaticMarkup(
    <ResearchMarkup
      html={
        '<section><script>alert(1)</script><p onmouseover="alert(1)">Source &amp; rules</p><a href="javascript:alert(1)">Unsafe</a><table><tbody><tr><td>0.00%</td></tr></tbody></table></section>'
      }
    />,
  );
  expect(html).not.toContain("<script");
  expect(html).not.toContain("onmouseover");
  expect(html).not.toContain("javascript:");
  expect(html).toContain("Source &amp; rules");
  expect(html).toContain("<td>0.00%</td>");
});
it("preserves form defaults and SVG icons for existing research controls", () => {
  const html = renderToStaticMarkup(
    <ResearchMarkup
      html={
        '<input type="checkbox" checked><select><option value="a">A</option><option value="b" selected>B</option></select><input value="2026-01-01"><svg viewBox="0 0 24 24"><path d="M0 0"/></svg>'
      }
    />,
  );
  expect(html).toContain("checked");
  expect(html).toContain("selected");
  expect(html).toContain("2026-01-01");
  expect(html).toContain('viewBox="0 0 24 24"');
});
it("renders only slots registered by the React page and keeps the safe surrounding markup", () => {
  const html = renderToStaticMarkup(
    <ResearchMarkup
      html={'<section><span class="action-slot" data-react-slot="settings" onmouseover="alert(1)">Old fallback</span><div data-react-slot="unknown"><a href="javascript:alert(1)">Reading fallback</a></div></section>'}
      slots={{settings: <button type="button">Reviewed React control</button>}}
    />,
  );
  expect(html).toContain('class="action-slot"');
  expect(html).toContain('<button type="button">Reviewed React control</button>');
  expect(html).not.toContain("Old fallback");
  expect(html).toContain("Reading fallback");
  expect(html).not.toContain("javascript:");
  expect(html).not.toContain("onmouseover");
});
it("does not treat inherited names or blocked tags as registered React slots", () => {
  const html = renderToStaticMarkup(
    <ResearchMarkup
      html={'<span data-react-slot="constructor">Normal content</span><script data-react-slot="settings">Unsafe content</script>'}
      slots={{settings: <button>Trusted control</button>}}
    />,
  );
  expect(html).toContain("Normal content");
  expect(html).not.toContain("Trusted control");
  expect(html).not.toContain("Unsafe content");
});
it("supports only the reviewed author actions without evaluating source strings", async () => {
  const { createRoot } = await import("react-dom/client"),
    { act } = await import("react"),
    { vi } = await import("vitest");
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const openAuthor = vi.fn(),
    closeAuthor = vi.fn();
  window.VaultRuntime = {
    openAuthor,
    closeAuthor,
  } as unknown as Window["VaultRuntime"];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () =>
    root.render(
      <ResearchMarkup
        html={
          '<button onclick="openGuru(\'tharp\', event)">Open</button><button onclick="closeGuru()">Close</button><button onclick="alert(1)">Rejected</button><a onclick="event.stopPropagation()" href="#/">Link</a><textarea value="source"></textarea><select multiple><optgroup><option value="a" selected>A</option></optgroup></select>'
        }
      />,
    ),
  );
  await act(async () => {
    host.querySelectorAll("button")[0].click();
    host.querySelectorAll("button")[1].click();
    host.querySelector("a")!.click();
  });
  expect(openAuthor).toHaveBeenCalledWith("tharp", expect.anything());
  expect(closeAuthor).toHaveBeenCalledOnce();
  expect(host.innerHTML).not.toContain("onclick");
  await act(async () => root.unmount());
  host.remove();
});
