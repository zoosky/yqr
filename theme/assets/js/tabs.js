// Tab switching for the [tabs] shortcode (Feature f145j).
// Handles click, keyboard navigation, group sync, localStorage persistence,
// and URL fragment activation. No framework dependencies.
(function () {
  var KEY = 'accent-tabs-';

  function activate(tab, panel, container) {
    container.querySelectorAll('.tabs-tab').forEach(function (t) {
      t.classList.remove('active');
      t.setAttribute('aria-selected', 'false');
    });
    container.querySelectorAll('.tabs-panel').forEach(function (p) {
      p.classList.remove('active');
    });
    tab.classList.add('active');
    tab.setAttribute('aria-selected', 'true');
    panel.classList.add('active');
  }

  function syncGroup(group, id) {
    document.querySelectorAll('.tabs[data-tab-group="' + group + '"]').forEach(function (c) {
      var t = c.querySelector('.tabs-tab[data-tab="' + id + '"]');
      if (t) {
        var p = c.querySelector('#tab-panel-' + id);
        if (p) activate(t, p, c);
      }
    });
    try { localStorage.setItem(KEY + group, id); } catch (_) {}
  }

  document.addEventListener('click', function (e) {
    var tab = e.target.closest('.tabs-tab');
    if (!tab) return;
    var container = tab.closest('.tabs');
    var id = tab.getAttribute('data-tab');
    var group = container.getAttribute('data-tab-group') || 'default';
    syncGroup(group, id);
  });

  document.addEventListener('keydown', function (e) {
    var tab = e.target.closest('.tabs-tab');
    if (!tab) return;
    var nav = tab.closest('.tabs-nav');
    var tabs = Array.from(nav.querySelectorAll('.tabs-tab'));
    var idx = tabs.indexOf(tab);
    var next = -1;
    if (e.key === 'ArrowRight') next = (idx + 1) % tabs.length;
    else if (e.key === 'ArrowLeft') next = (idx - 1 + tabs.length) % tabs.length;
    if (next >= 0) {
      tabs[next].focus();
      tabs[next].click();
      e.preventDefault();
    }
  });

  // Restore from URL hash or localStorage on page load.
  function restore() {
    var hash = location.hash.replace('#', '');
    document.querySelectorAll('.tabs').forEach(function (c) {
      var group = c.getAttribute('data-tab-group') || 'default';
      var saved;
      try { saved = localStorage.getItem(KEY + group); } catch (_) {}
      var id = (hash && hash.startsWith('tab-')) ? hash.replace('tab-', '') : saved;
      if (id) {
        var t = c.querySelector('.tabs-tab[data-tab="' + id + '"]');
        var p = c.querySelector('#tab-panel-' + id);
        if (t && p) activate(t, p, c);
      }
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', restore);
  } else {
    restore();
  }
})();
