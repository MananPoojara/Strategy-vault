import type {SourceAccountView} from "../services/source-account";

export function SourceAccountNotice({account}: {account: SourceAccountView}) {
  return <aside className="portfolio-basis" data-source-account-notice aria-label="Protected Impulse research account">
    <strong>Protected Impulse · fixed research application</strong>
    <p>Next actual open · completed-week EMA26 · SafeZone 10/2. The fixed worksheet lookback does not reproduce
      the book’s discretionary turning-point resets.</p>
    <p>Protection and the 2% per-trade / 6% monthly risk limits stay active. Closed-return display settings
      do not change trades, allocation, the account curve or its risk statistics.</p>
    <details><summary>Account and execution basis</summary><p>{account.basis}</p></details>
  </aside>;
}
