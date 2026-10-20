import type {PristineView} from './services/pristine-report';
/// <reference types="vite/client" />
import type { Account } from "./services/api";
export interface VaultRuntimeContract {
  bootstrap(account: Account): Promise<void>;
  commit(): void;
  cleanup(): void;
  activateReportTab?(): void;
  patch(patch: Record<string, string>): void;
  pristineCurrent?(data:unknown, view:PristineView):boolean;
  exportPristine?(data:unknown, view:PristineView,kind:'workbook'|'csv'):Promise<void>;
  changeTheme(theme: string): void;
  openAuthor(id: string, event?: unknown): void;
  closeAuthor(): void;
  actions(): string;
  themes: readonly string[][];
  saved(): {
    key: string;
    id: string;
    name: string;
    tk: string;
    hash: string;
    at: string;
    book: string;
    symbol: string;
  }[];
  removeSaved(key: string): void;
}
declare global {
  interface Window {
    VAULT_APP: boolean;
    VAULT_USER: Account["user"];
    VAULT_SETTINGS: Account["settings"];
    VAULT_SESSION_READY: Promise<Account>;
    VaultRuntime: VaultRuntimeContract;
  }
}
