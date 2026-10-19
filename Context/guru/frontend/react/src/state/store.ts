import type { ReadingModel } from "../pages/ReadingPage";
import type { Route } from "../app/routes";
import type { LibraryMarket } from "../services/library-market";
import type { OwnerIssue as SummaryIssue } from "../services/summary-loader";
import type { SearchAuthor } from "../services/library-search";
import type {SourceAccountView} from "../services/source-account";
import type {RuleVisualModel} from "../services/rule-evidence";
export interface ViewSnapshot {
  revision: number;
  loading: boolean;
  html: string;
  error: string | null;
  route: Route | null;
  status: string;
  state: Record<string, string>;
  home: HomeModel | null;
  reading: ReadingModel | null;
  strategyAccount?: SourceAccountView | null;
  strategyRules?: RuleVisualModel | null;
}
export interface PageInfo {
  page: number;
  pages: number;
  total: number;
  key: string;
  label: string;
}
export interface BookCard {
  scope?: string;
  id: string;
  kind: string;
  title: string;
  author: string;
  summary: string;
  year: string;
  categories: string[];
  href: string;
  live: boolean;
}
export interface AuthorCard {
  curated: boolean;
  id: string;
  name: string;
  href: string;
  photo: string;
  initials: string;
  tagline: string;
  preview: string;
  entries: number;
  catalogue: number;
  reading: number;
  families: number;
  available: number;
  availableContext?: string;
  contextChanged?: boolean;
  openLabel?: string;
  status: string;
  statusTitle: string;
  statusText: string;
  sharpe: string | null;
}
export interface HomeModel {
  summaryIssues?: SummaryIssue[];
  market: LibraryMarket;
  implementedMethods: number;
  enabledVariants: number;
  implementedBooks: number;
  curatedAuthors: number;
  scope: string;
  inventoryGroups: number;
  inventoryEntries: number;
  authors: number;
  entries: number;
  families: number;
  symbol: string;
  symbolCount: number;
  basket: boolean;
  authorCards: AuthorCard[];
  searchAuthors?: SearchAuthor[];
  bookCards: BookCard[];
  page: PageInfo;
  query: string;
  category: string;
  view: string;
  indices: { ticker: string; name: string }[];
  etfs?: { ticker: string; name: string }[];
  baskets: { id: string; name: string; symbols: string[] }[];
  symbols: { ticker: string; name: string }[];
  usAvailable: boolean;
  guilloche: string;
  connorsLink: string;
  strategiesLink: string;
}
export function createStore() {
  let generation = 0;
  let snapshot: ViewSnapshot = {
    revision: 0,
    loading: true,
    html: "",
    error: null,
    route: null,
    status: "Loading",
    state: {},
    home: null,
    reading: null,
  };
  const listeners = new Set<() => void>();
  const notify = () => listeners.forEach((listener) => listener());
  return {
    getSnapshot: () => snapshot,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    begin: (_path: string) => {
      generation++;
      snapshot = { ...snapshot, loading: true, error: null };
      notify();
      return generation;
    },
    commit: (ticket: number, patch: Partial<ViewSnapshot>) => {
      if (ticket !== generation) return false;
      snapshot = {
        ...snapshot,
        ...patch,
        revision: ticket,
        loading: false,
        error: null,
      };
      notify();
      return true;
    },
    fail: (ticket: number, error: string) => {
      if (ticket !== generation) return false;
      snapshot = {
        ...snapshot,
        revision: ticket,
        loading: false,
        error,
        html: "",
        home: null,
        reading: null,
        strategyAccount: null,
        strategyRules: null,
      };
      notify();
      return true;
    },
  };
}
export const vaultStore = createStore();
