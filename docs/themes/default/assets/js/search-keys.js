// Search overlay — opens on button click or "/" shortcut, closes on Escape or backdrop click.
//
// The built-in search bundle (/_search/search.js) wires up ArrowUp/ArrowDown
// navigation, Enter to follow, and result rendering. This file manages the
// overlay lifecycle on top of that.
(function () {
    "use strict";

    var overlay, trigger, input;

    function isEditable(el) {
        if (!el) return false;
        var tag = el.tagName ? el.tagName.toLowerCase() : "";
        return (
            tag === "input" ||
            tag === "textarea" ||
            tag === "select" ||
            el.isContentEditable === true
        );
    }

    function openOverlay() {
        if (!overlay) return;
        overlay.removeAttribute("hidden");
        trigger && trigger.setAttribute("aria-expanded", "true");
        if (input) {
            input.focus();
            var len = input.value.length;
            try { input.setSelectionRange(len, len); } catch (_) {}
        }
    }

    function closeOverlay() {
        if (!overlay) return;
        overlay.setAttribute("hidden", "");
        trigger && trigger.setAttribute("aria-expanded", "false");
        trigger && trigger.focus();
    }

    document.addEventListener("DOMContentLoaded", function () {
        overlay  = document.getElementById("search-overlay");
        trigger  = document.getElementById("search-trigger");
        input    = overlay && overlay.querySelector(".acms-search-input");
        var backdrop = document.getElementById("search-overlay-backdrop");

        if (trigger) trigger.addEventListener("click", openOverlay);
        if (backdrop) backdrop.addEventListener("click", closeOverlay);
    });

    document.addEventListener("keydown", function (e) {
        if (e.key === "/" && !e.ctrlKey && !e.metaKey && !e.altKey) {
            if (isEditable(document.activeElement)) return;
            e.preventDefault();
            openOverlay();
            return;
        }
        if (e.key === "Escape" && overlay && !overlay.hasAttribute("hidden")) {
            closeOverlay();
        }
    });
})();
