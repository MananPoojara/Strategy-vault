import {
  useEffect,
  useLayoutEffect,
  useState,
  useSyncExternalStore,
} from "react";
import { vaultStore, type ViewSnapshot } from "../state/store";
import {
  getAccount,
  saveSettings,
  ApiError,
  type Account,
} from "../services/api";
import { AppHeader } from "../components/AppHeader";
import { BookLoader } from "../components/BookLoader";
import { ResearchMarkup } from "../components/ResearchMarkup";
import { HomePage } from "../pages/HomePage";
import { SavedPage } from "../pages/SavedPage";
import {
  AuthorPage,
  StrategiesPage,
  StrategyPage,
  OptimizationPage,
} from "../pages/ResearchPages";
import { ReadingPage } from "../pages/ReadingPage";
import { resolveRoute } from "./routes";
import { customPages } from "./custom-pages";
import "../compat/runtime.js";
let initialization: Promise<Account> | undefined;
function initialize() {
  return (initialization ??= (async () => {
    const account = await getAccount();
    window.VAULT_USER = account.user;
    window.VAULT_SETTINGS = account.settings;
    if (!location.hash && account.settings.lastView)
      history.replaceState(
        null,
        "",
        location.pathname + location.search + account.settings.lastView,
      );
    return account;
  })());
}
function ResearchRoute({ snapshot }: { snapshot: ViewSnapshot }) {
  useLayoutEffect(() => {
    window.VaultRuntime.commit();
    return () => window.VaultRuntime.cleanup();
  }, [snapshot.revision]);
  const pages = {
    home: HomePage,
    author: AuthorPage,
    strategies: StrategiesPage,
    strategy: StrategyPage,
    optimization: OptimizationPage,
    saved: SavedPage,
  };
  const route = snapshot.route || resolveRoute(location.hash);
  const Page = pages[route.name as keyof typeof pages];
  return (
    <div className="research-page" data-react-page={route.name}>
      <ResearchMarkup html={window.VaultRuntime.actions()} />
      {snapshot.reading ? (
        <ReadingPage model={snapshot.reading} />
      ) : Page ? (
        <Page snapshot={snapshot} />
      ) : (
        <div className="empty">
          <b>Page not found</b>
          <a href="#/">Return to the library</a>
        </div>
      )}
    </div>
  );
}
export function App() {
  const snapshot = useSyncExternalStore(
    vaultStore.subscribe,
    vaultStore.getSnapshot,
  );
  const [account, setAccount] = useState<Account | null>(null),
    [sessionError, setSessionError] = useState(""),
    [hash, setHash] = useState(location.hash);
  useEffect(() => {
    let mounted = true;
    initialize()
      .then(async (account) => {
        if (!mounted) return;
        setAccount(account);
        window.VaultRuntime.changeTheme?.(account.settings.theme || "aqua");
      })
      .catch((error) => {
        if (!mounted) return;
        if (error instanceof ApiError && error.status === 401)
          location.replace(
            "/login.html?next=" +
              encodeURIComponent(
                location.pathname + location.search + location.hash,
              ),
          );
        else setSessionError(error.message || "Could not load your account.");
      });
    return () => {
      mounted = false;
    };
  }, []);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    const changed = () => {
      setHash(location.hash);
      if (account) {
        clearTimeout(timer);
        timer = setTimeout(() => {
          saveSettings({ lastView: location.hash || "#/" }).catch(() => {
            /* Navigation must not depend on preference storage. */
          });
        }, 500);
      }
    };
    addEventListener("hashchange", changed);
    return () => {
      removeEventListener("hashchange", changed);
      clearTimeout(timer);
    };
  }, [account]);
  const Custom = customPages.find(
    (page) => page.path === resolveRoute(hash).path,
  )?.component;
  // A custom page brings its own data: the catalogues and library wait for the first page that reads them.
  useEffect(() => {
    if (account && !Custom) void window.VaultRuntime.bootstrap(account);
  }, [account, Custom]);
  return (
    <>
      <AppHeader
        key={account?.user.id || "loading"}
        account={account}
        snapshot={snapshot}
      />
      <main className="page" id="main" aria-busy={snapshot.loading && !Custom && !sessionError && !snapshot.error}>
        {sessionError ? (
          <div className="empty" role="alert">
            <b>Could not load your account</b>
            {sessionError}
          </div>
        ) : Custom ? (
          <Custom />
        ) : snapshot.error ? (
          <div className="empty" role="alert">
            <b>Could not load the vault</b>
            {snapshot.error}
          </div>
        ) : snapshot.revision && snapshot.route ? (
          <ResearchRoute key={snapshot.revision} snapshot={snapshot} />
        ) : (
          <BookLoader message="Loading the vault…" />
        )}
      </main>
      {snapshot.loading && snapshot.revision > 0 && !Custom && !sessionError && !snapshot.error && (
        <div className="flipov" id="flipov">
          <BookLoader message="Loading the selected research…" />
        </div>
      )}
    </>
  );
}
