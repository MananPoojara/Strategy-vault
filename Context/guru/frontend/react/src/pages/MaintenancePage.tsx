/**
 * Static maintenance surface shared by the application and the standalone page.
 * The host supplies the font faces, includes maintenanceStyles, and attaches the
 * availability check to maintenance-retry and maintenance-status.
 */
export default function MaintenancePage() {
  return (
    <div className="sv-maintenance">
      <header className="sv-maintenance__header" aria-label="Strategy Vault">
        <span className="sv-maintenance__wordmark">STRATEGY VAULT</span>
        <span className="sv-maintenance__state">
          <svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
            <path d="M7 5v10M13 5v10" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          </svg>
          Update in progress
        </span>
      </header>

      <main className="sv-maintenance__main" aria-labelledby="maintenance-heading">
        <div className="sv-maintenance__content">
          <h1 id="maintenance-heading">
            We’re updating<br />Strategy Vault.
          </h1>
          <div className="sv-maintenance__mechanism">
            <svg viewBox="0 0 360 360" fill="none" aria-hidden="true">
              <path className="sv-maintenance__tracks" d="M0 180h36m288 0h36M180 0v36m0 288v36" />
              <circle className="sv-maintenance__tracks" cx="180" cy="180" r="144" />
              <circle className="sv-maintenance__ticks" cx="180" cy="180" r="128" strokeDasharray="1 15.75" />
              <circle className="sv-maintenance__tracks" cx="180" cy="180" r="104" />
              <g className="sv-maintenance__orbit sv-maintenance__orbit--outer">
                <circle className="sv-maintenance__arc" cx="180" cy="180" r="144" strokeDasharray="122 104 122 557" strokeLinecap="round" />
                <circle cx="180" cy="36" r="5" fill="currentColor" />
                <circle cx="180" cy="324" r="3" fill="currentColor" />
              </g>
              <g className="sv-maintenance__orbit sv-maintenance__orbit--inner">
                <circle className="sv-maintenance__arc sv-maintenance__arc--inner" cx="180" cy="180" r="104" strokeDasharray="76 88 76 413" strokeLinecap="round" />
                <circle cx="76" cy="180" r="4" fill="currentColor" />
              </g>
              <circle className="sv-maintenance__core" cx="180" cy="180" r="65" />
              <circle className="sv-maintenance__tracks" cx="180" cy="180" r="47" />
              <g className="sv-maintenance__rotor">
                <path d="M180 142v20m0 36v20m-38-38h20m36 0h20" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                <circle cx="180" cy="180" r="18" stroke="currentColor" strokeWidth="2" />
                <circle cx="180" cy="180" r="5" fill="currentColor" />
              </g>
              <circle className="sv-maintenance__signal sv-maintenance__signal--in" cx="15" cy="180" r="3" fill="currentColor" />
              <circle className="sv-maintenance__signal sv-maintenance__signal--out" cx="330" cy="180" r="3" fill="currentColor" />
            </svg>
            <label className="sv-maintenance__motion-control">
              <input type="checkbox" />
              Pause animation
            </label>
          </div>
          <div className="sv-maintenance__details">
            <p className="sv-maintenance__intro">
              Strategy Vault is temporarily unavailable while we roll out an update.
            </p>
            <p className="sv-maintenance__guidance">
              Keep this page open. We’ll check automatically and return you to your
              original page when it’s ready.
            </p>
            <button id="maintenance-retry" className="sv-maintenance__retry" type="button">
              <svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
                <path d="M16 6.5A6.5 6.5 0 1 0 16.4 13M16 3v4h-4" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              Check now
            </button>
            <noscript>
              <p className="sv-maintenance__noscript">
                Automatic checks need JavaScript. Refresh this page to check again.
              </p>
            </noscript>
          </div>
        </div>

        <div className="sv-maintenance__availability">
          <span className="sv-maintenance__indicator" aria-hidden="true" />
          <p id="maintenance-status" role="status" aria-live="polite" aria-atomic="true">
            Checking for availability…
          </p>
        </div>
      </main>
    </div>
  );
}

export const maintenanceStyles = `
body:has(.sv-maintenance) {
  margin: 0;
  background: #f3f5f7;
}
.sv-maintenance {
  --maintenance-paper: #f3f5f7;
  --maintenance-ink: #182330;
  --maintenance-muted: #536170;
  --maintenance-action: #2f5a8f;
  --maintenance-rule: #ccd4dd;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  min-height: 100svh;
  width: 100%;
  padding: 32px clamp(24px, 5vw, 80px);
  background: var(--maintenance-paper);
  color: var(--maintenance-ink);
  font: 400 16px/1.6 "IBM Plex Sans", sans-serif;
  font-synthesis: none;
  -webkit-font-smoothing: antialiased;
  color-scheme: light;
}
.sv-maintenance *, .sv-maintenance *::before, .sv-maintenance *::after {
  box-sizing: border-box;
}
.sv-maintenance ::selection {
  background: #d8e4f2;
  color: #182330;
}
.sv-maintenance__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  width: 100%;
  max-width: 1184px;
  margin-inline: auto;
  padding-bottom: 28px;
  border-bottom: 1px solid var(--maintenance-rule);
}
.sv-maintenance__wordmark {
  font: 800 18px/1.3 "Archivo", "IBM Plex Sans", sans-serif;
  font-stretch: 125%;
  letter-spacing: -0.015em;
  white-space: nowrap;
}
.sv-maintenance__state {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--maintenance-action);
  font-size: 14px;
  line-height: 1.5;
  white-space: nowrap;
}
.sv-maintenance__state svg {
  flex: none;
  width: 20px;
  height: 20px;
}
.sv-maintenance__main {
  display: flex;
  flex: 1;
  flex-direction: column;
  justify-content: center;
  width: 100%;
  max-width: 1184px;
  margin-inline: auto;
  padding-block: 48px;
}
.sv-maintenance__content {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
  align-items: center;
  gap: 28px clamp(40px, 6vw, 88px);
  animation: sv-maintenance-enter 280ms cubic-bezier(0.23, 1, 0.32, 1) both;
}
.sv-maintenance h1 {
  grid-column: 1;
  grid-row: 1;
  align-self: end;
  margin: 0;
  color: var(--maintenance-ink);
  font: 600 clamp(44px, 4.4vw, 64px)/1.08 "IBM Plex Sans", sans-serif;
  letter-spacing: -0.035em;
  text-wrap: balance;
}
.sv-maintenance__details {
  grid-column: 1;
  grid-row: 2;
  align-self: start;
  max-width: 44ch;
}
.sv-maintenance__mechanism {
  grid-column: 2;
  grid-row: 1 / span 2;
  justify-self: center;
  width: min(100%, 360px);
  color: var(--maintenance-action);
}
.sv-maintenance__mechanism svg {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 1;
  overflow: visible;
}
.sv-maintenance__motion-control {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: fit-content;
  min-height: 32px;
  margin: 12px auto 0;
  color: var(--maintenance-muted);
  font-size: 14px;
  cursor: pointer;
}
.sv-maintenance__motion-control input {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: var(--maintenance-action);
  cursor: pointer;
}
.sv-maintenance__motion-control input:focus-visible {
  outline: 2px solid var(--maintenance-action);
  outline-offset: 3px;
}
.sv-maintenance__mechanism:has(input:checked) :is(.sv-maintenance__orbit, .sv-maintenance__rotor, .sv-maintenance__signal) {
  animation-play-state: paused;
}
.sv-maintenance__tracks { stroke: #c7d3e0; stroke-width: 1; }
.sv-maintenance__ticks { stroke: #a9bbd0; stroke-width: 5; }
.sv-maintenance__arc { stroke: currentColor; stroke-width: 3; }
.sv-maintenance__arc--inner { stroke-width: 2; }
.sv-maintenance__core { fill: #e7edf4; stroke: #b8c9dc; stroke-width: 1; }
.sv-maintenance__orbit, .sv-maintenance__rotor {
  transform-box: view-box;
  transform-origin: 50% 50%;
}
.sv-maintenance__orbit--outer { animation: sv-maintenance-orbit 18s linear infinite; }
.sv-maintenance__orbit--inner { animation: sv-maintenance-orbit 12s linear infinite reverse; }
.sv-maintenance__rotor { animation: sv-maintenance-orbit 8s linear infinite; }
.sv-maintenance__signal { animation: sv-maintenance-signal 2.4s cubic-bezier(0.77, 0, 0.175, 1) infinite; }
.sv-maintenance__signal--out { animation-delay: -1.2s; }
@keyframes sv-maintenance-orbit {
  to { transform: rotate(360deg); }
}
@keyframes sv-maintenance-signal {
  0% { transform: translateX(0); opacity: 0; }
  25%, 65% { opacity: 1; }
  100% { transform: translateX(18px); opacity: 0; }
}
@keyframes sv-maintenance-enter {
  from { transform: translateY(8px); opacity: .35; }
  to { transform: translateY(0); opacity: 1; }
}
.sv-maintenance__intro {
  margin: 0;
  color: var(--maintenance-ink);
}
.sv-maintenance__guidance {
  margin: 16px 0 28px;
  color: var(--maintenance-muted);
}
.sv-maintenance__retry {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-width: 154px;
  min-height: 48px;
  margin: 0;
  padding: 11px 20px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: var(--maintenance-action);
  color: #ffffff;
  font: 600 16px/1.5 "IBM Plex Sans", sans-serif;
  cursor: pointer;
  text-align: center;
  transition: background-color 140ms ease;
  -webkit-tap-highlight-color: transparent;
}
.sv-maintenance__retry svg {
  flex: none;
  width: 20px;
  height: 20px;
}
.sv-maintenance__retry:hover:not(:disabled) {
  background: #244a78;
}
.sv-maintenance__retry:active:not(:disabled) {
  background: #1d3d64;
}
.sv-maintenance__retry:focus-visible {
  outline: 2px solid var(--maintenance-action);
  outline-offset: 4px;
}
.sv-maintenance__retry:disabled {
  background: #526c8a;
  color: #ffffff;
  cursor: wait;
}
.sv-maintenance__availability {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 40px;
  padding-top: 22px;
  border-top: 1px solid var(--maintenance-rule);
  color: var(--maintenance-muted);
  font-size: 14px;
}
.sv-maintenance__indicator {
  flex: none;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--maintenance-action);
}
.sv-maintenance__availability p {
  margin: 0;
  overflow-wrap: anywhere;
}
.sv-maintenance__noscript {
  margin: 20px 0 0;
  color: var(--maintenance-muted);
  font-size: 14px;
}
@media (max-width: 800px) {
  .sv-maintenance__main { padding-block: 48px; }
  .sv-maintenance__content { grid-template-columns: 1fr; gap: 24px; }
  .sv-maintenance h1 { margin-top: 0; font-size: clamp(40px, 7vw, 56px); }
  .sv-maintenance__mechanism { grid-column: 1; grid-row: 2; width: 240px; }
  .sv-maintenance__details { grid-row: 3; }
  .sv-maintenance__details { max-width: 48ch; }
  .sv-maintenance__availability { margin-top: 32px; }
}
@media (max-width: 480px) {
  .sv-maintenance { padding-top: 24px; padding-bottom: 24px; }
  .sv-maintenance__header { flex-direction: column; align-items: flex-start; gap: 16px; padding-bottom: 24px; }
  .sv-maintenance__main { justify-content: flex-start; padding-top: 32px; padding-bottom: 24px; }
  .sv-maintenance h1 { font-size: clamp(34px, 10vw, 44px); }
  .sv-maintenance__mechanism { width: 192px; }
  .sv-maintenance__guidance { margin-bottom: 24px; }
}
@media (prefers-reduced-motion: reduce) {
  .sv-maintenance__retry { transition: none; }
  .sv-maintenance__content, .sv-maintenance__orbit, .sv-maintenance__rotor, .sv-maintenance__signal { animation: none; }
  .sv-maintenance__signal { opacity: 0; }
  .sv-maintenance__motion-control { display: none; }
}
@media (forced-colors: active) {
  .sv-maintenance__retry { border-color: ButtonText; }
  .sv-maintenance__indicator { background: CanvasText; }
}
`;
