// Feature f164: click-to-copy handler for the `copy` heading-anchor preset.
// Intercepts clicks on `.heading-anchor` elements, writes the canonical
// permalink to the clipboard, and shows a transient toast. Falls back to
// default link navigation when clipboard API is unavailable.
(function () {
  "use strict";

  if (!navigator.clipboard) return;

  let toastEl = null;
  let toastTimer = null;

  function showToast(text) {
    if (!toastEl) {
      toastEl = document.createElement("div");
      toastEl.className = "heading-anchor-toast";
      toastEl.setAttribute("role", "status");
      toastEl.setAttribute("aria-live", "polite");
      document.body.appendChild(toastEl);
    }
    toastEl.textContent = text;
    toastEl.classList.add("visible");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () {
      toastEl.classList.remove("visible");
    }, 1500);
  }

  document.addEventListener("click", function (e) {
    const a = e.target.closest(".heading-anchor");
    if (!a) return;
    const href = a.getAttribute("href");
    if (!href || href.charAt(0) !== "#") return;
    e.preventDefault();
    const url = location.origin + location.pathname + location.search + href;
    navigator.clipboard.writeText(url).then(
      function () {
        showToast("Link copied");
        history.replaceState(null, "", location.pathname + location.search + href);
      },
      function () {
        location.hash = href.slice(1);
      },
    );
  });
})();
