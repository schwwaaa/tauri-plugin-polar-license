/*!
 * tauri-plugin-polar-license — drop-in UI
 *
 * Two components, no framework, no build step:
 *   • the activation GATE: a full-window overlay shown until the app is licensed
 *   • the license PANEL: status, computer, key, last check, "Release this machine"
 *
 * Usage (classic script; works with `withGlobalTauri: true` and a strict CSP):
 *
 *   <link rel="stylesheet" href="polar-license.css">
 *   <script src="polar-license.js"></script>
 *   <script>
 *     PolarLicenseUI.mount({
 *       appName: 'Voxel Feedback Volume',
 *       onLicensed: (status) => startApp(),        // after activation or at launch
 *       onReleased: () => quitApp(),               // after "Release this machine"
 *       onQuit: () => quitApp(),                   // gate's Quit button (optional)
 *     });
 *     document.querySelector('#gear').addEventListener('click', () => PolarLicenseUI.openPanel());
 *   </script>
 *
 * With a bundler, pass your own invoke:  mount({ invoke }) where
 *   import { invoke } from '@tauri-apps/api/core';
 *
 * Everything is built with textContent (never innerHTML), so no value from
 * Polar or the user can inject markup. Styling: see polar-license.css.
 */
(function (global) {
  'use strict';

  var CMD = {
    status: 'plugin:polar-license|status',
    activate: 'plugin:polar-license|activate',
    release: 'plugin:polar-license|release',
    launchCheck: 'plugin:polar-license|launch_check',
  };

  var opts = null;
  var gateEl = null;
  var panelEl = null;
  var releaseTimer = null;

  // ── helpers ───────────────────────────────────────────────────────────
  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  }
  function invoke(cmd, args) {
    return opts.invoke(cmd, args || {});
  }
  function errorText(e) {
    if (e && typeof e === 'object' && e.message) return e.message;
    return String(e);
  }
  function fmtDate(iso) {
    if (!iso) return '—';
    var d = new Date(iso);
    return isNaN(d) ? iso : d.toLocaleString();
  }
  function t(key, fallback) {
    return (opts.text && opts.text[key]) || fallback;
  }

  // ── gate ──────────────────────────────────────────────────────────────
  function buildGate(status) {
    var overlay = el('div', 'pl-overlay pl-gate');
    var dialog = el('div', 'pl-dialog');
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    dialog.setAttribute('aria-labelledby', 'pl-gate-title');

    dialog.appendChild(el('p', 'pl-eyebrow', t('gateEyebrow', 'License required')));
    var title = el('h2', 'pl-title', t('gateTitle', opts.appName ? 'Activate ' + opts.appName : 'Activate this computer'));
    title.id = 'pl-gate-title';
    dialog.appendChild(title);
    dialog.appendChild(el('p', 'pl-text', t('gateBody',
      'Paste the license key from your purchase email. This is a one-time step on this computer; after that, the app starts without needing the internet.')));

    var form = el('form', 'pl-form');
    var input = el('input', 'pl-input');
    input.type = 'text';
    input.autocomplete = 'off';
    input.spellcheck = false;
    input.placeholder = 'XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX';
    input.setAttribute('aria-label', 'License key');
    var message = el('p', 'pl-message');
    message.setAttribute('aria-live', 'polite');
    var row = el('div', 'pl-row');
    var activateBtn = el('button', 'pl-button pl-primary', t('activate', 'Activate'));
    activateBtn.type = 'submit';
    row.appendChild(activateBtn);
    if (opts.onQuit) {
      var quit = el('button', 'pl-button', t('quit', 'Quit'));
      quit.type = 'button';
      quit.addEventListener('click', function () { opts.onQuit(); });
      row.appendChild(quit);
    }
    form.appendChild(input);
    form.appendChild(message);
    form.appendChild(row);
    dialog.appendChild(form);

    if (status.environment === 'sandbox') {
      dialog.appendChild(el('p', 'pl-note', 'Test build — using the Polar sandbox.'));
    }
    if (!status.configured) {
      message.className = 'pl-message pl-error';
      message.textContent = 'This build has no Polar IDs yet. Add them to the plugin configuration and rebuild.';
    }

    input.addEventListener('input', function () { message.textContent = ''; message.className = 'pl-message'; });
    form.addEventListener('submit', function (ev) {
      ev.preventDefault();
      if (activateBtn.disabled) return;
      activateBtn.disabled = true;
      activateBtn.textContent = t('activating', 'Activating…');
      message.className = 'pl-message pl-busy';
      message.textContent = t('contacting', 'Contacting the license server…');
      invoke(CMD.activate, { key: input.value })
        .then(function (s) {
          closeGate();
          if (opts.onLicensed) opts.onLicensed(s);
        })
        .catch(function (e) {
          message.className = 'pl-message pl-error';
          message.textContent = errorText(e);
        })
        .then(function () {
          activateBtn.disabled = false;
          activateBtn.textContent = t('activate', 'Activate');
        });
    });

    overlay.appendChild(dialog);
    setTimeout(function () { input.focus(); }, 0);
    return overlay;
  }

  function closeGate() {
    if (gateEl) { gateEl.remove(); gateEl = null; }
  }

  // ── panel ─────────────────────────────────────────────────────────────
  function row(dl, label, value) {
    dl.appendChild(el('dt', null, label));
    var dd = el('dd', null, value);
    dl.appendChild(dd);
    return dd;
  }

  function closePanel() {
    if (releaseTimer) { clearTimeout(releaseTimer); releaseTimer = null; }
    if (panelEl) { panelEl.remove(); panelEl = null; }
    document.removeEventListener('keydown', onKey);
  }
  function onKey(ev) { if (ev.key === 'Escape') closePanel(); }

  function openPanel() {
    if (!opts) throw new Error('PolarLicenseUI.mount() must be called first');
    closePanel();
    Promise.all([invoke(CMD.status), invoke(CMD.launchCheck)]).then(function (res) {
      var s = res[0], check = res[1];
      var overlay = el('div', 'pl-overlay pl-panel');
      overlay.addEventListener('click', function (ev) { if (ev.target === overlay) closePanel(); });
      var dialog = el('div', 'pl-dialog');
      dialog.setAttribute('role', 'dialog');
      dialog.setAttribute('aria-modal', 'true');
      dialog.setAttribute('aria-labelledby', 'pl-panel-title');
      dialog.appendChild(el('p', 'pl-eyebrow', opts.appName || 'About'));
      var title = el('h2', 'pl-title', t('panelTitle', 'License'));
      title.id = 'pl-panel-title';
      dialog.appendChild(title);

      var dl = el('dl', 'pl-details');
      row(dl, 'Status', s.licensed ? 'Activated' : 'Not activated');
      row(dl, 'This computer', s.label || '—');
      row(dl, 'Key', s.displayKey || '—');
      row(dl, 'Activated', fmtDate(s.activatedAt));
      row(dl, 'Server', s.environment === 'sandbox' ? 'Polar sandbox (test)' : 'Polar');
      var checkDd = row(dl, 'Last check', check.message);
      checkDd.className = 'pl-check pl-check-' + check.state;
      dialog.appendChild(dl);

      var message = el('p', 'pl-message');
      message.setAttribute('aria-live', 'polite');
      var actions = el('div', 'pl-row');
      if (s.licensed) {
        var releaseBtn = el('button', 'pl-button pl-danger-outline', t('release', 'Release this machine'));
        releaseBtn.type = 'button';
        releaseBtn.addEventListener('click', function () {
          // Two clicks within 4 s, so a stray click mid-show can't release.
          if (!releaseTimer) {
            releaseBtn.textContent = t('releaseConfirm', 'Click again to release');
            releaseBtn.classList.add('pl-armed');
            releaseTimer = setTimeout(function () {
              releaseTimer = null;
              releaseBtn.textContent = t('release', 'Release this machine');
              releaseBtn.classList.remove('pl-armed');
            }, 4000);
            return;
          }
          clearTimeout(releaseTimer); releaseTimer = null;
          releaseBtn.disabled = true;
          releaseBtn.textContent = t('releasing', 'Releasing…');
          invoke(CMD.release).then(function () {
            message.className = 'pl-message pl-ok';
            message.textContent = t('released', 'Released. This key can now be activated on another computer.');
            if (opts.onReleased) setTimeout(opts.onReleased, 1200);
          }).catch(function (e) {
            message.className = 'pl-message pl-error';
            message.textContent = errorText(e);
            releaseBtn.disabled = false;
            releaseBtn.textContent = t('release', 'Release this machine');
            releaseBtn.classList.remove('pl-armed');
          });
        });
        actions.appendChild(releaseBtn);
      }
      var close = el('button', 'pl-button', t('close', 'Close'));
      close.type = 'button';
      close.addEventListener('click', closePanel);
      actions.appendChild(close);
      dialog.appendChild(message);
      dialog.appendChild(actions);
      if (s.licensed) {
        dialog.appendChild(el('p', 'pl-note', t('releaseHelp',
          'Releasing frees this computer\'s activation so the key can be used on another one. You\'ll need the key again to use the app here.')));
      }
      overlay.appendChild(dialog);
      document.body.appendChild(overlay);
      panelEl = overlay;
      document.addEventListener('keydown', onKey);
      close.focus();

      // Still checking? Update the row when the answer arrives.
      if (check.state === 'pending') {
        var tries = 0;
        var timer = setInterval(function () {
          if (!panelEl || ++tries > 30) return clearInterval(timer);
          invoke(CMD.launchCheck).then(function (c) {
            if (c.state === 'pending') return;
            clearInterval(timer);
            checkDd.textContent = c.message;
            checkDd.className = 'pl-check pl-check-' + c.state;
          });
        }, 1000);
      }
    });
  }

  // ── mount ─────────────────────────────────────────────────────────────
  function mount(options) {
    opts = Object.assign({}, options || {});
    if (!opts.invoke) {
      var core = global.__TAURI__ && global.__TAURI__.core;
      if (!core) throw new Error('PolarLicenseUI: no invoke(). Set "withGlobalTauri": true or pass { invoke }.');
      opts.invoke = core.invoke;
    }
    return invoke(CMD.status).then(function (status) {
      if (status.licensed) {
        if (opts.onLicensed) opts.onLicensed(status);
      } else {
        closeGate();
        gateEl = buildGate(status);
        document.body.appendChild(gateEl);
      }
      return status;
    });
  }

  global.PolarLicenseUI = { mount: mount, openPanel: openPanel, closePanel: closePanel };
})(typeof window !== 'undefined' ? window : this);
