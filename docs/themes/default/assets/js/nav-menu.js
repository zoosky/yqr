/**
 * Collapsible header navigation for narrow viewports.
 *
 * The header nav is a single flex row: site title, page links, tools. That
 * stops fitting once there are more than a few pages -- at 390px the row
 * overflowed the viewport and the document scrolled sideways.
 *
 * This script opts the nav into a disclosure pattern by adding the
 * `has-toggle` class. The CSS only collapses the menu when that class is
 * present, so with JavaScript unavailable the links simply wrap onto a
 * second row and stay reachable -- the toggle button is never revealed and
 * nothing depends on a click that cannot happen.
 */
(function () {
  'use strict';

  var nav = document.querySelector('header nav');
  var toggle = document.getElementById('nav-menu-toggle');
  var menu = document.getElementById('nav-menu');
  if (!nav || !toggle || !menu) return;

  // Signals to the stylesheet that the collapsed behaviour is safe to apply.
  nav.classList.add('has-toggle');

  function setOpen(open) {
    nav.classList.toggle('menu-open', open);
    toggle.setAttribute('aria-expanded', open ? 'true' : 'false');
  }

  function isOpen() {
    return toggle.getAttribute('aria-expanded') === 'true';
  }

  toggle.addEventListener('click', function () {
    setOpen(!isOpen());
  });

  // Escape closes and returns focus to the button, so keyboard users are
  // never left with focus inside a panel they have dismissed.
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape' && isOpen()) {
      setOpen(false);
      toggle.focus();
    }
  });

  // A click outside the header dismisses the panel.
  document.addEventListener('click', function (e) {
    if (isOpen() && !nav.contains(e.target)) setOpen(false);
  });

  // Following a link navigates away, but closing first keeps the state sane
  // for in-page anchors and for browser back/forward restoring the page.
  menu.addEventListener('click', function (e) {
    if (e.target.closest('a')) setOpen(false);
  });

  // Widening past the breakpoint restores the full row; leaving the panel
  // flagged open would then show it expanded over the desktop layout.
  var wide = window.matchMedia('(min-width: 781px)');
  var onChange = function (e) {
    if (e.matches) setOpen(false);
  };
  if (wide.addEventListener) wide.addEventListener('change', onChange);
  else if (wide.addListener) wide.addListener(onChange);
})();
