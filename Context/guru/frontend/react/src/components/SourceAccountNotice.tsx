import {sourceAccountCopy,type SourceAccountView} from "../services/source-account";

export function SourceAccountNotice({account}: {account: SourceAccountView}) {
  const copy=sourceAccountCopy(account);
  if (copy.bandy) return <aside className="portfolio-basis" data-source-account-notice aria-label="Bandy fractional cash research account">
    <strong>Bandy · fractional cash research account</strong>
    <p>{copy.sizing}</p><p>Fixed reviewed settings on the selected instrument. Closed-return display settings do not change accepted positions,
      allocation, the cash-account curve or its risk statistics. Gross retains the same accepted quantities and fills. Open marks pay entry costs only;
      pending exits and hypothetical liquidation costs are not executed charges.</p>
    <p>Exposure includes observed sessions from entry through exit. Drawdown duration counts observed sessions, not calendar days or intraday time.
      Zero-return closed trades join non-wins in loss statistics. Gross trade fractions use actual net entry equity.</p>
    <details><summary>Account and execution basis</summary><p>{account.basis}</p></details>
  </aside>;
  return <aside className="portfolio-basis" data-source-account-notice aria-label="Protected Impulse research account">
    <strong>Protected Impulse · fixed research application</strong>
    <p>Next actual open · completed-week EMA26 · SafeZone 10/2. The fixed worksheet lookback does not reproduce
      the book’s discretionary turning-point resets.</p>
    <p>Protection and the 2% per-trade / 6% monthly risk limits stay active. Closed-return display settings
      do not change trades, allocation, the account curve or its risk statistics.</p>
    <details><summary>Account and execution basis</summary><p>{account.basis}</p></details>
  </aside>;
}
