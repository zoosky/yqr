// Reference copy of the dev-mode diagram overlay script.
//
// At runtime, Accent injects the overlay inline from
// `src/server/dev_reload.rs::DIAGRAM_OVERLAY_SCRIPT` when
// `dev.browser_reload: true` and the `diagrams` cargo feature is on.
// This file is NOT loaded by the default theme; it is shipped as a
// reference for theme authors who want to study or customise the
// overlay behaviour. The live source of truth is
// `src/server/dev_reload.rs` -- keep this file in sync with the
// const there or it will drift.
//
// Behaviour: scans `.diagram-wrapper[data-source-file]` on
// DOMContentLoaded, attaches click handlers that route to the
// editor via `window.accentDevOverlay.lspJumpTo(file, line, col)`
// when available, otherwise falls back to a `file://...#L<line>`
// URL the OS routes to the user's registered handler.
//
// Feature f161j

(function () {
  if (window.__acDiagOverlay) return;
  window.__acDiagOverlay = true;
  function jumpTo(file, line, col) {
    var bridge = window.accentDevOverlay;
    if (bridge && typeof bridge.lspJumpTo === 'function') {
      try { bridge.lspJumpTo(file, line, col); return; } catch (e) {}
    }
    if (!file) return;
    var url = 'file://' + (file.charAt(0) === '/' ? file : '/' + file);
    if (line) url += '#L' + line;
    window.open(url);
  }
  function bindOne(wrapper) {
    var file = wrapper.getAttribute('data-source-file');
    if (!file) return;
    wrapper.querySelectorAll('[data-source-line]').forEach(function (el) {
      if (el.getAttribute('data-acdiag-bound')) return;
      el.setAttribute('data-acdiag-bound', '1');
      el.style.cursor = 'pointer';
      el.addEventListener('click', function (ev) {
        ev.stopPropagation();
        var line = el.getAttribute('data-source-line');
        var col = el.getAttribute('data-source-col');
        jumpTo(file, line ? parseInt(line, 10) : 0, col ? parseInt(col, 10) : 0);
      });
    });
    if (!wrapper.getAttribute('data-acdiag-wrapper-bound')) {
      wrapper.setAttribute('data-acdiag-wrapper-bound', '1');
      wrapper.addEventListener('click', function (ev) {
        if (ev.target.closest('[data-source-line]')) return;
        jumpTo(file, 0, 0);
      });
    }
  }
  function scan() {
    document.querySelectorAll('.diagram-wrapper[data-source-file]').forEach(bindOne);
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', scan);
  } else {
    scan();
  }
})();
