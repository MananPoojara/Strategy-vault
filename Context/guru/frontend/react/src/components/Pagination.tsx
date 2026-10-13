import type { PageInfo } from "../state/store";
export function Pagination({ page }: { page: PageInfo }) {
  if (page.pages < 2) return null;
  return (
    <nav className="library-load" aria-label={`${page.label} pages`}>
      <button
        className="dbtn sm"
        disabled={page.page <= 1}
        onClick={() =>
          window.VaultRuntime.patch({ [page.key]: String(page.page - 1) })
        }
      >
        ← Previous
      </button>
      <span>
        Page {page.page} of {page.pages} · {page.total} entries
      </span>
      <button
        className="dbtn sm"
        disabled={page.page >= page.pages}
        onClick={() =>
          window.VaultRuntime.patch({ [page.key]: String(page.page + 1) })
        }
      >
        Next 24 {page.label} →
      </button>
    </nav>
  );
}
