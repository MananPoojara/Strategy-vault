/** Server labels and fixed copy, verbatim (spec §2, §3, §6). Grep here before writing any UI text. */
export const LAB_LABEL = "User-defined indicator strategy · simulated · not a book method";
export const TEMPLATE_LABEL = "Book template · declared adaptation into lab rules · not a verification";
export const FIDELITY_LABEL = "Template fidelity · display only · not a verification";
export const PROVISIONAL_NOTE = "Provisional · TradingView export · internal research only, not publishable to YT Strategies";
export const UNAVAILABLE_PREFIX = "Customize not available — ";
export const variantLabel = (book: string, method: string) => `Customized from ${book} · ${method} · user variant · simulated`;
export const unavailableText = (reason: string) => (reason.startsWith(UNAVAILABLE_PREFIX) ? reason : UNAVAILABLE_PREFIX + reason);

export const COPY = {
  longSide: "Long side", shortSide: "Short side",
  longEntryTitle: "Long entry · all of these", longExitTitle: "Long exit · any of these",
  shortEntryTitle: "Short entry · all of these", shortExitTitle: "Short exit · any of these",
  addMenu: "+ Add…",
  addOptions: { cond: "Condition", all: "All of (group)", any: "Any of (group)", not: "Not (negate)", at_least: "At least K of the last M bars", then: "Sequence (A then B within N bars)" },
  nodeMenu: { wrap_all: "Wrap in All of", wrap_any: "Wrap in Any of", wrap_not: "Negate (Not)", unwrap: "Unwrap", up: "Move up", down: "Move down" },
  heldTopOnly: "Bars held works only in a top-level exit condition (not inside a group, range, trend or formula).",
  tooDeep: "These rules are nested too deeply (the engine reads at most 16 levels). Unwrap a group or a formula.",
  caps: (top: string, leaves: number, maxLeaves: number, nodes: number, maxNodes: number, kb: string) =>
    `${top} · comparisons ${leaves}/${maxLeaves} · rule parts ${nodes}/${maxNodes} · ${kb} of 7.5 KB`,
  sessionDatesHint: "These rules read Sessions left, which uses this instrument's future session dates (never prices). Results carry an adaptation note.",
  oppositeHelp: {
    reverse: "Close the position and open the other side in one fill (the AmiBroker and TradingView default).",
    exit: "Only close the position; a new trade needs the next signal.",
    ignore: "Keep the position; the other side's signal does nothing until the position is closed.",
  },
  bothFlat: "If both sides signal on the same bar while flat, no trade is taken.",
  intrabarHelp: {
    stop_first: "When a stop and a target both sit inside one daily bar, the stop is assumed to fill first (AmiBroker's priority; the cautious choice).",
    ohlc_path: "TradingView's path: open, then the nearer of high and low, then the other, then close; the first level the path reaches fills first.",
  },
  delayOnlyNextOpen: "Fill delay applies to next-open fills only.",
  stopLimitHint: "A limit set as a distance sits below the signal close for a long (above for a short), so this order fills only after a gap through the stop. For the usual stop-limit, set the limit with At (at or above the stop).",
  minBarsHint: "Stops and targets stay active.",
  moneyCostsHere: "Money mode: fees, slippage and taxes come from the money settings.",
  needsMoney: "Shares and position-value sizing need money mode.",
  riskNeedsStop: "Risk per trade needs a stop-loss on every side.",
  borrowRequired: "US short positions in money mode need a declared borrow cost (0 is allowed).",
  scaleInWithTp: "Scale-in cannot be combined with take-profits.",
  studyOnly: "Study only · not executable", unitStudy: "Unit study",
  riskEvents: "Risk events",

  lead: "Pick indicators from the library, chart them on any market, turn them into entry and exit rules, then backtest and optimize. Saved strategies can be submitted to YT Strategies.",
  chartHint: "Click a library item to add it · click an indicator chip to open its Parameters",
  noChips: "No indicators on the chart. Pick one from the library.",
  chartNote: "Indicators are calculated from the full history, then the visible range is shown.",
  inProgress: (tf: string) => `The last ${tf === "W" ? "weekly" : tf === "M" ? "monthly" : "period"} bar is still in progress; its values can change until it closes.`,
  wmRule: "Higher-timeframe values are used only after that period closes (unless a timing option says otherwise).",
  notChecked: "Not yet checked against TradingView.",
  rulesHint: "values from completed bars only",
  entryTitle: "Entry · all of these",
  exitTitle: "Exit · any of these",
  addCondition: "+ Add condition",
  emptyEntry: "Add at least one entry condition.",
  keptAsIs: "Kept as is: this page cannot edit this rule type yet.",
  startMoved: (date: string) => `Results start on ${date}, not the requested start: earlier opens on this instrument are not reliable enough for next-open fills.`,
  noTrades: "No trades. The entry conditions never lined up on this instrument.",
  researcherOnly: "Researcher access is required for optimization.",
  optimizeLimit: "The engine stops an optimization after 300 s.",
  tickOne: "Tick at least one setting",
  splitLine: (pct: number, date: string) => `Out-of-sample: the last ${pct}% (from ${date}) is held back · settings are ranked on the earlier part only`,
  splitNote: "In-sample and held-back trade counts can differ from the full run: trades that cross the split date are cut at the boundary. A strong ranking score with a weak held-back result usually means curve fitting.",
  noEligible: (min: number) => `No setting had enough trades to rank (at least ${min}). Nothing is ranked.`,
  emptyOptimize: "Run the optimization to rank settings. Results show the ranking period and the held-back period side by side.",
  runFirst: "Run the current rules first; the engine checks the rules only when they run.",
  provisionalSubmit: "Provisional data cannot be published. Rerun on licensed data first.",
  savedEmpty: "Nothing saved yet. Name the current rules above and save them.",
  keptAfterReload: (version: number) => `Your rules on screen are kept. Save again to replace version ${version}.`,
  deleteConfirm: (name: string) => `Delete "${name}"? This cannot be undone.`,
  submitted: "Submitted. An admin reviews it before it appears in YT Strategies.",
  customizeGlOnly: "Customize runs on India and US book methods only.",
  fidelityNothing: "No trades on this window, so there is nothing to compare.",
  ytIntro: "Published lab strategies. Each one is a simulated, user-defined rule set reviewed for presentation — not a book method and not a verification.",
  ytEmpty: "Nothing is published yet.",
  tooLong: "These rules are too long to send. Remove a condition or shorten settings.",
} as const;

export const EXPAND_FIRST_REFUSED = "expandFirst would show the period's final value from its first day (lookahead); use offset >= 1, the period open, or expand last";
/** Pre-run mirrors of lab/model.rs notes; after a run the response's execution_model.notes are shown instead. */
export const MODEL_NOTE = {
  index: "An index cannot be traded; returns are a study of index levels, not of an ETF or futures contract.",
  indiaShort: "Hypothetical — overnight short positions are not executable in the Indian cash segment (they need F&O or SLB borrowing).",
  usShortUnit: "Short positions: borrow cost is not modelled in unit mode.",
  crypto: "CRYPTO:* is a TradingView index composite, not an exchange order book.",
  cryptoShort: "Hypothetical — shorting spot needs a margin borrow or perpetual data with funding, which is not staged.",
  cfd: "Provisional CFD quotes; CFD contract terms (size, financing) are not admitted.",
} as const;
export const MONEY_BLOCKED = {
  index: "Money mode needs a tradable quantity: an index is never priced as a stand-in for an ETF or futures contract.",
  crypto: "Money mode is not available: the crypto series is a TradingView index composite, not an exchange order book.",
  cfd: "Money mode is not available: CFD contract terms are not admitted.",
  indiaShort: "Money mode is not available for short positions on India cash instruments: overnight shorts need an F&O contract or an SLB borrow model, and neither is admitted.",
} as const;
export const FUTURES_NOTE = "Futures: the contract model exists, but no futures contract data (contract chain with expiries, lot sizes and tick size) is admitted, so futures runs answer “Data needed”. The index or spot series is never used as a stand-in.";
export const MONEY_BASIS_HINT = "User-declared costs; not a verified statutory schedule.";
export const OPERAND_KIND_LABELS: Record<string, string> = {
  price: "Price", indicator: "Indicator", calendar: "Calendar", number: "Number", bars_held: "Bars held",
  arith: "Arithmetic", pct: "% difference", abs: "Absolute value", ref: "N bars ago", window: "Rolling window", change: "Change over N bars",
  compress: "Higher-timeframe aggregate", bars_since: "Bars since", value_when: "Value when", since: "Highest / lowest since", count: "Count of bars",
  formula: "My indicator",
};
export const CALENDAR_OUTPUTS: { id: string; out: string; label: string }[] = [
  { id: "session", out: "tdom", label: "Trading day of the month" }, { id: "session", out: "tdow", label: "Trading day of the week" },
  { id: "session", out: "tdoq", label: "Trading day of the quarter" }, { id: "session", out: "tdoy", label: "Trading day of the year" },
  { id: "session", out: "week_of_month", label: "Week of the month" }, { id: "session", out: "quarter", label: "Quarter (1–4)" },
  { id: "sessions_left", out: "m", label: "Sessions left in the month · future dates" },
  { id: "sessions_left", out: "q", label: "Sessions left in the quarter · future dates" },
  { id: "sessions_left", out: "y", label: "Sessions left in the year · future dates" },
  { id: "session", out: "weekdays_left_m", label: "Weekdays left in the month" }, { id: "session", out: "weekdays_left_q", label: "Weekdays left in the quarter" },
  { id: "session", out: "weekdays_left_y", label: "Weekdays left in the year" },
  { id: "calendar", out: "dow", label: "Day of the week (0 Sun … 6 Sat)" }, { id: "calendar", out: "day", label: "Day of the month" },
  { id: "calendar", out: "month", label: "Month (1–12)" }, { id: "calendar", out: "year", label: "Year" }, { id: "calendar", out: "doy", label: "Day of the year" },
];
export const TF_LABELS: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly", Q: "Quarterly", Y: "Yearly" };
export const ENTRY_KIND_LABELS: Record<string, string> = {
  next_open: "Next open", close: "Close", limit: "Limit", stop: "Stop", stop_limit: "Stop-limit", reverse: "Reversal", roll: "Contract roll",
  high: "Bar high", low: "Bar low", average: "Bar average",
};
