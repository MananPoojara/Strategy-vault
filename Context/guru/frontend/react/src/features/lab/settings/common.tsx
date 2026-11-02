import type { ReactNode } from "react";
import { modeOf, moneyBlocked, supportsMoney, type InstrumentContext } from "../execution-model";
import type { Catalogue, Spec } from "../types";

/** The one reason beside an AmiBroker option the lab engine does not offer (the same words on every tab). */
export const NOT_IN_LAB = "Not available in the lab yet";
export const CASH_ONLY = "Needs money mode: a stock, ETF, index or futures contract (see Backtest parameters).";
/** The full reason is under Backtest parameters; repeated beside every cash field it would drown the tab. */
export const CASH_BLOCKED = "Needs money mode, which this instrument cannot use (see Backtest parameters).";
export const ONE_INSTRUMENT = "One instrument only: not used in a portfolio.";

/** The short plain-English reason beside an option that is visible but off; nothing when it applies. */
export const Why = ({ text, title }: { text?: string | null | false; title?: string }) => text ? <span className="cap" data-lab-why title={title}>{text}</span> : null;

/** A long explanation folded behind its one-line gist; `data-*` attributes go on the fold so tests and styles find the whole note. */
export const Note = ({ gist, children, ...data }: { gist: ReactNode; children?: ReactNode } & { [k: `data-${string}`]: string | boolean }) => children
  ? <details className="lab-note" {...data}><summary className="cap">{gist}</summary><p className="cap">{children}</p></details>
  : <p className="cap" {...data}>{gist}</p>;

/** A labelled group inside a settings tab: the label in a left column so related controls read as one block.
 *  `why`: the reasons its greyed fields are off, said once for the group (duplicates and empties dropped) instead of on every row. */
export const SettingGroup = ({ title, why = [], children }: { title: string; why?: (string | null | false | undefined)[]; children: ReactNode }) => {
  const reasons = [...new Set(why.filter((w): w is string => !!w))];
  return <section className="lab-sgroup" aria-label={title}><h4>{title}</h4><div>
    {reasons.length > 0 && <p className="cap lab-sgroup-why">Greyed out: {reasons.map((r) => <Why key={r} text={r} />)}</p>}
    {children}</div></section>;
};

/** Why a cash-only field is off: null in money mode, where it stays editable even when blocked so a loaded value can be fixed (the run reports the block). */
export const cashWhy = (spec: Spec, catalogue: Catalogue, instrument: InstrumentContext): string | null => modeOf(spec) === "money" ? null
  : moneyBlocked(instrument, spec) ? CASH_BLOCKED : !supportsMoney(catalogue, instrument.kind) ? "Money mode is not supported by this engine yet." : CASH_ONLY;
