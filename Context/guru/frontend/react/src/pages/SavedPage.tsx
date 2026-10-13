import { ResearchMarkup } from "../components/ResearchMarkup";
export function SavedPage() {
  const list = window.VaultRuntime.saved();
  return (
    <section className="box">
      <div className="box-h">
        Saved strategies{" "}
        <small>{list.length} saved · stored in this browser</small>
      </div>
      {list.length ? (
        <div className="saved-list">
          {list.map((item) => (
            <div className="saved-row" key={item.key}>
              <span aria-hidden="true">☆</span>
              <div>
                <a className="t" href={item.hash}>
                  {item.name}
                </a>
                <small>
                  {item.book}
                  {item.book ? " · " : ""}
                  {item.symbol} · saved {item.at}
                </small>
              </div>
              <button
                type="button"
                className="ibtn"
                data-unsave={item.key}
                title="Remove from Saved"
                aria-label="Remove from Saved"
                onClick={(event) => {
                  event.stopPropagation();
                  window.VaultRuntime.removeSaved(item.key);
                }}
              >
                <ResearchMarkup
                  html={
                    '<svg viewBox="0 0 24 24" class="ic ic-line"><path d="M6 4h12v17l-6-4-6 4z"/></svg>'
                  }
                />
              </button>
            </div>
          ))}
        </div>
      ) : (
        <p className="rnote">
          Nothing saved yet. Open a strategy and press Save to keep it here,
          with its index and settings.
        </p>
      )}
    </section>
  );
}
