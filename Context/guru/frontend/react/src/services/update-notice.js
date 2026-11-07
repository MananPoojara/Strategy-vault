(function () {
  'use strict';
  var build = document.querySelector('meta[name="vault-build"]');
  var preview = new URLSearchParams(location.search).get('update-preview') === '1';
  if ((!preview && (!build || !build.content)) || document.documentElement.hasAttribute('data-vault-maintenance')) return;
  var checking = false, announced = '', dialog;
  var refreshIcon = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5"/><path d="M6.1 7a7 7 0 0 1 11.6-1L20 9M4 15l2.3 3A7 7 0 0 0 17.9 17"/></svg>';
  function show() {
    if (!dialog) {
      var style = document.createElement('style');
      style.textContent = `
        .vault-update{box-sizing:border-box;width:min(560px,calc(100% - 32px));max-height:calc(100dvh - 32px);margin:auto;padding:40px;border:1px solid #e1e5eb;border-radius:20px;background:#fff;color:#17212d;box-shadow:0 24px 80px #14223526;font:400 18px/1.65 "IBM Plex Sans",system-ui,sans-serif;text-align:left;overflow:auto}
        .vault-update::backdrop{background:rgb(23 29 38 / .30);backdrop-filter:blur(7px)}
        .vault-update[open]{animation:vault-update-in .22s ease-out}
        .vault-update .vault-update-icon{display:grid;place-items:center;width:56px;height:56px;margin-bottom:26px;border-radius:16px;background:#edf2f9;color:#315b8f}
        .vault-update svg{width:24px;height:24px;flex-shrink:0}
        .vault-update h2{margin:0 0 14px;font:600 26px/1.25 "IBM Plex Sans",system-ui,sans-serif;letter-spacing:-.02em;color:inherit}
        .vault-update p{margin:0 0 28px;font:inherit;color:#566170}
        .vault-update button{box-sizing:border-box;display:flex;align-items:center;justify-content:center;gap:10px;width:100%;min-height:52px;padding:12px 18px;border:0;border-radius:12px;background:#315b8f;color:#fff;font:600 18px/1.5 "IBM Plex Sans",system-ui,sans-serif;cursor:pointer}
        .vault-update button:hover{background:#264b79}
        .vault-update button:focus-visible{outline:2px solid #315b8f;outline-offset:3px}
        .vault-update .vault-update-help{margin:20px 0 0;font-size:14px;line-height:1.7;color:#647080}
        .vault-update kbd{font:600 13px/1.5 "IBM Plex Sans",system-ui,sans-serif;white-space:nowrap}
        @keyframes vault-update-in{from{opacity:0;transform:translateY(8px)}to{opacity:1;transform:translateY(0)}}
        @media(max-width:480px){.vault-update{padding:28px 24px}.vault-update h2{font-size:24px}.vault-update p{font-size:16px}}
        @media(prefers-reduced-motion:reduce){.vault-update[open]{animation:none}}
        @media(forced-colors:active){.vault-update,.vault-update button{border:1px solid ButtonText}}
      `;
      document.head.appendChild(style);
      dialog = document.createElement('dialog');
      dialog.className = 'vault-update';
      dialog.setAttribute('aria-labelledby', 'vault-update-heading');
      dialog.setAttribute('aria-describedby', 'vault-update-description vault-update-help');
      dialog.innerHTML = '<div class="vault-update-icon">' + refreshIcon + '</div><h2 id="vault-update-heading">An update is ready</h2><p id="vault-update-description">Strategy Vault has changed since you opened this tab. Refresh to load the latest version before continuing.</p><button type="button">' + refreshIcon + 'Refresh now</button><p id="vault-update-help" class="vault-update-help">Refreshing clears unsaved changes in this tab. Press <kbd>Esc</kbd> to keep working.<br>If the page still looks old, use <kbd>Ctrl + Shift + R</kbd> on Windows/Linux or <kbd>Cmd + Shift + R</kbd> on Mac.</p>';
      dialog.querySelector('button').addEventListener('click', function () {
        if (preview) { var url = new URL(location.href); url.searchParams.delete('update-preview'); location.replace(url.href); }
        else location.reload();
      });
      document.body.appendChild(dialog);
    }
    if (!dialog.open) dialog.showModal();
  }
  async function check() {
    if (checking || document.hidden) return;
    checking = true;
    try {
      var response = await fetch('/vendor/app-version.json', { cache: 'no-store', credentials: 'same-origin' });
      if (!response.ok) return;
      var state = await response.json();
      if (typeof state.build !== 'string' || !/^[a-f0-9]{64}$/.test(state.build)) return;
      if (state.build !== build.content && state.build !== announced) { announced = state.build; show(); }
    } catch (_) { /* A failed availability check must not interrupt the current page. */ }
    finally { checking = false; }
  }
  if (preview) { show(); return; }
  document.addEventListener('visibilitychange', function () { if (!document.hidden) void check(); });
  window.addEventListener('pageshow', function () { void check(); });
  setInterval(check, 30000);
  void check();
}());
