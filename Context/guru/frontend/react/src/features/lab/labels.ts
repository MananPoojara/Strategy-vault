/** Server labels and fixed copy, verbatim (spec §2, §3, §6). Grep here before writing any UI text. */
export const LAB_LABEL = "User-defined indicator strategy · simulated · not a book method";
export const TEMPLATE_LABEL = "Book template · declared adaptation into lab rules · not a verification";
export const FIDELITY_LABEL = "Template fidelity · display only · not a verification";
export const PROVISIONAL_NOTE = "Provisional · TradingView export · internal research only, not publishable to YT Strategies";
export const UNAVAILABLE_PREFIX = "Customize not available — ";
export const variantLabel = (book: string, method: string) => `Customized from ${book} · ${method} · user variant · simulated`;
export const unavailableText = (reason: string) => (reason.startsWith(UNAVAILABLE_PREFIX) ? reason : UNAVAILABLE_PREFIX + reason);

export const COPY = {
  lead: "Pick indicators from the library, chart them on any market, turn them into entry and exit rules, then backtest and optimize. Saved strategies can be submitted to YT Strategies.",
  chartHint: "Click a library item to add it · edit settings in the chips",
  noChips: "No indicators on the chart. Pick one from the library.",
  chartNote: "Indicators are calculated from the full history, then the visible range is shown.",
  inProgress: (tf: string) => `The last ${tf === "W" ? "weekly" : tf === "M" ? "monthly" : "period"} bar is still in progress; its values can change until it closes.`,
  wmRule: "Weekly and monthly values are used only after that bar closes.",
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
