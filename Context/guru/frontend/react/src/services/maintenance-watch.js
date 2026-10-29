(function () {
  'use strict';
  var maintenanceDocument = document.documentElement.hasAttribute('data-vault-maintenance');
  var preview = maintenanceDocument && new URLSearchParams(location.search).get('preview') === '1';
  var checking = false, redirecting = false;
  function status(text) { var el = document.getElementById('maintenance-status'); if (el) el.textContent = text; }
  async function check() {
    if (preview) { status('Maintenance preview · automatic reconnect is enabled during an update.'); return; }
    if (checking || redirecting || (document.hidden && !maintenanceDocument)) return;
    checking = true;
    var button = document.getElementById('maintenance-retry');
    if (button) button.disabled = true;
    try {
      var response = await fetch('/__maintenance/status', { cache: 'no-store', credentials: 'same-origin' });
      if (!response.ok) throw new Error('Availability check failed');
      var state = await response.json();
      if (typeof state.maintenance !== 'boolean') throw new Error('Availability status unavailable');
      if (state.maintenance) {
        if (maintenanceDocument) status('The update is in progress. We’ll check again automatically.');
        else { redirecting = true; location.reload(); }
      } else if (maintenanceDocument) {
        redirecting = true;
        status('Strategy Vault is ready. Taking you back…');
        if (location.pathname === '/maintenance.html') location.replace('/'); else location.reload();
      } else status('Strategy Vault is available.');
    } catch (_) { status('We couldn’t reach the site. We’ll keep checking automatically.'); }
    finally { checking = false; if (button) button.disabled = false; }
  }
  document.addEventListener('click', function (event) {
    var target = event.target;
    if (target && target.closest && target.closest('#maintenance-retry')) { event.preventDefault(); void check(); }
  });
  document.addEventListener('visibilitychange', function () { if (!document.hidden) void check(); });
  window.addEventListener('pageshow', function () { void check(); });
  setInterval(check, 8000);
  void check();
}());
