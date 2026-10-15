import { MAX_PANES, MAX_PRICE_OVERLAYS } from "./catalogue";
import { canonicalJson, defaultSpec } from "./spec-model";
import type { EditorState } from "./SavedStrategies";
import type { LabQuery } from "./url-state";
import type { Chip } from "./Workbench";
import type { ChartRange, Indicator, LabStrategy, Market, SavedMarket, Spec } from "./types";

export interface LabState {
  market: Market; ticker: string; tf: string; range: ChartRange; spec: Spec; chips: Chip[];
  placements: Record<string, "price" | "pane">; nextChip: number; row: LabStrategy | null;
  lastRun: string | null; tried: number;
}
export type LabAction =
  | { type: "market"; market: Market } | { type: "ticker"; ticker: string } | { type: "tf"; tf: string } | { type: "range"; range: ChartRange }
  | { type: "spec"; spec: Spec } | { type: "open"; row: LabStrategy | null; spec: Spec } | { type: "row"; row: LabStrategy }
  | { type: "chip-add"; indicator: Indicator } | { type: "chip-set"; key: string; params: Record<string, number> } | { type: "chip-remove"; key: string }
  | { type: "ran"; key: string; combos: number } | { type: "tried"; n: number };

export const runKey = (spec: Spec, market: SavedMarket, ticker: string) => canonicalJson([spec, market, ticker]);

/** The page passes regionFallback only when the hash has no `region` (then the research pages' region wins). */
export function initialLabState(q: LabQuery, regionFallback?: string): LabState {
  const market: Market = regionFallback === "us" ? "us" : q.region;
  return { market, ticker: q.tk, tf: q.tf, range: q.range, spec: defaultSpec(), chips: [], placements: {}, nextChip: 1, row: null, lastRun: null, tried: 0 };
}

export function labReducer(state: LabState, action: LabAction): LabState {
  switch (action.type) {
    case "market":
      return action.market === state.market ? state : { ...state, market: action.market, ticker: "", chips: [], placements: {}, row: null, lastRun: null };
    case "ticker": return { ...state, ticker: action.ticker, lastRun: null };
    case "tf": return { ...state, tf: action.tf };
    case "range": return { ...state, range: action.range };
    case "spec": return { ...state, spec: action.spec };
    case "open": return { ...state, row: action.row, spec: action.spec, lastRun: null };
    case "row": return { ...state, row: action.row };
    case "chip-add": {
      const { indicator } = action;
      const key = `c${state.nextChip}`;
      const params = Object.fromEntries(indicator.params.map((p) => [p.key, p.default]));
      let chips = [...state.chips, { key, id: indicator.id, params }];
      const placements = { ...state.placements, [key]: indicator.placement };
      const cap = indicator.placement === "price" ? MAX_PRICE_OVERLAYS : MAX_PANES;
      const same = chips.filter((c) => placements[c.key] === indicator.placement);
      if (same.length > cap) chips = chips.filter((c) => c.key !== same[0].key);
      return { ...state, chips, placements, nextChip: state.nextChip + 1 };
    }
    case "chip-set": return { ...state, chips: state.chips.map((c) => (c.key === action.key ? { ...c, params: action.params } : c)) };
    case "chip-remove": return { ...state, chips: state.chips.filter((c) => c.key !== action.key) };
    case "ran": return { ...state, lastRun: action.key, tried: state.tried + 1 };
    case "tried": return { ...state, tried: state.tried + action.n };
  }
}

export function editorState(state: LabState, market: SavedMarket): EditorState {
  const unchanged = !!state.row && state.row.instrument === state.ticker && canonicalJson(state.row.spec) === canonicalJson(state.spec);
  return { spec: state.spec, market, ticker: state.ticker, row: state.row, unchanged,
    ranFresh: state.lastRun === runKey(state.spec, market, state.ticker) };
}
