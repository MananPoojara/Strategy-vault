/** Preserve the vault's existing book/page-flip artwork at the React page boundary. */
export function BookLoader({message = "Loading…"}: {message?: string}) {
  return <div className="bload" role="status" aria-live="polite" data-book-loader>
    <div className="bflip" aria-hidden="true">
      <i className="bcv l" /><i className="bcv r" />
      <i className="bpg" /><i className="bpg" /><i className="bpg" />
    </div>
    <p>{message}</p>
  </div>;
}
