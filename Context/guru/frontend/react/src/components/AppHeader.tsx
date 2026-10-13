import { useState } from "react";
import type { Account } from "../services/api";
import { saveSettings, signOut } from "../services/api";
import type { ViewSnapshot } from "../state/store";
import { customPages } from "../app/custom-pages";
const themes = [
  ["midnight", "Midnight", "#0d1727", "#4eade8"],
  ["forest", "Forest", "#eef4ed", "#357552"],
  ["sand", "Sand", "#f6f0e5", "#bd6935"],
  ["plum", "Plum", "#f6edf3", "#8c477d"],
  ["cobalt", "Cobalt", "#edf2fc", "#315ad6"],
  ["steel", "Steel", "#f3f3f3", "#2f5a8f"],
  ["aqua", "Aqua", "#ffffff", "#10adbc"],
  ["lagoon", "Lagoon", "#e9f8fa", "#00798a"],
  ["glacier", "Glacier", "#f8fcfd", "#3cc0c9"],
];
export function AppHeader({
  account,
  snapshot,
}: {
  account: Account | null;
  snapshot: ViewSnapshot;
}) {
  const [theme, setTheme] = useState(account?.settings.theme || "aqua"),
    [signingOut, setSigningOut] = useState(false),
    [message, setMessage] = useState("");
  const active = snapshot.route?.name;
  const savedCount = window.VaultRuntime?.saved().length || 0;
  return (
    <>
      <header className="topbar">
        <a className="wordmark" href="#/">
          STRATEGY VAULT
        </a>
        <nav className="topnav">
          <a
            href="#/"
            data-r="home"
            className={
              ["home", "author", "reading"].includes(active || "") ? "on" : ""
            }
          >
            Guru's
          </a>
          <a
            href="#/strategies"
            data-r="all"
            className={active === "strategies" ? "on" : ""}
          >
            All strategies
          </a>
          <a
            href="#/saved"
            data-r="saved"
            className={active === "saved" ? "on" : ""}
          >
            Saved
            <span className="cnt" id="saved-count" hidden={!savedCount}>
              {savedCount}
            </span>
          </a>
          {customPages
            .filter((page) => page.showInNavigation)
            .map((page) => (
              <a href={`#${page.path}`} key={page.path}>
                {page.label}
              </a>
            ))}
          <a className="off">
            YT Strategies<em>soon</em>
          </a>
        </nav>
        <div className="status" id="status">
          {snapshot.status}
        </div>
        <div className="themes" id="themes" aria-label="Colour theme">
          <span>Theme</span>
          {themes.map(([key, label, base, accent]) => (
            <button
              type="button"
              key={key}
              data-theme={key}
              title={label}
              aria-label={`${label} theme`}
              className={theme === key ? "on" : ""}
              style={{
                background: `linear-gradient(135deg,${base} 50%,${accent} 50%)`,
              }}
              onClick={async () => {
                setTheme(key);
                window.VaultRuntime?.changeTheme(key);
                try {
                  await saveSettings({ theme: key });
                } catch {
                  setMessage(
                    "Theme changed on this device; saving to your account failed.",
                  );
                }
              }}
            />
          ))}
        </div>
        {account && (
          <div className="account-tools">
            <span className="account-name">
              {account.user.name} · {account.user.role}
            </span>
            {account.user.role === "admin" && (
              <a href="/admin.html">Accounts</a>
            )}
            <button
              type="button"
              disabled={signingOut}
              onClick={async () => {
                setSigningOut(true);
                try {
                  await signOut();
                  location.replace("/login.html");
                } catch {
                  setSigningOut(false);
                  setMessage("Could not sign out. Please try again.");
                }
              }}
            >
              Sign out
            </button>
          </div>
        )}
      </header>
      {message && (
        <div className="account-msg" role="status">
          {message}
          <button onClick={() => setMessage("")} aria-label="Dismiss message">
            ×
          </button>
        </div>
      )}
    </>
  );
}
