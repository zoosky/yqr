/**
 * Accent CMS Island Loader
 *
 * Discovers <accent-island> elements and schedules hydration based on
 * the data-hydrate strategy attribute. Island components register
 * themselves via AccentIslands.register(name, initFn).
 *
 * Hydration strategies:
 *   load    - Immediate execution
 *   idle    - requestIdleCallback (setTimeout(200) fallback)
 *   visible - IntersectionObserver (falls back to load)
 *   media   - matchMedia with data-media query
 */
(function () {
    "use strict";

    var registry = {};
    var pending = [];

    function hydrate(el) {
        var name = el.dataset.component;
        if (!name) return;
        if (el.getAttribute("data-hydrated") === "true") return;

        var initFn = registry[name];
        if (!initFn) {
            // Component not yet registered; queue for later
            if (pending.indexOf(el) === -1) pending.push(el);
            return;
        }

        var props = {};
        if (el.dataset.props) {
            try {
                props = JSON.parse(el.dataset.props);
            } catch (err) {
                return;
            }
        }

        try {
            initFn(el, props);
            el.setAttribute("data-hydrated", "true");
        } catch (err) {
            // Hydration failed; element retains server-rendered fallback
        }
    }

    function hydrateOnIdle(el) {
        if ("requestIdleCallback" in window) {
            requestIdleCallback(function () { hydrate(el); });
        } else {
            setTimeout(function () { hydrate(el); }, 200);
        }
    }

    function hydrateOnVisible(el) {
        if (!("IntersectionObserver" in window)) {
            hydrate(el);
            return;
        }
        var observer = new IntersectionObserver(function (entries) {
            for (var i = 0; i < entries.length; i++) {
                if (entries[i].isIntersecting) {
                    hydrate(entries[i].target);
                    observer.unobserve(entries[i].target);
                }
            }
        });
        observer.observe(el);
    }

    function hydrateOnMedia(el) {
        var query = el.dataset.media;
        if (!query) return;
        var mql = window.matchMedia(query);
        if (mql.matches) {
            hydrate(el);
        } else {
            mql.addEventListener("change", function handler(e) {
                if (e.matches) {
                    hydrate(el);
                    mql.removeEventListener("change", handler);
                }
            });
        }
    }

    function scheduleHydration(el) {
        var strategy = el.dataset.hydrate || "idle";
        switch (strategy) {
            case "load":
                hydrate(el);
                break;
            case "idle":
                hydrateOnIdle(el);
                break;
            case "visible":
                hydrateOnVisible(el);
                break;
            case "media":
                hydrateOnMedia(el);
                break;
            default:
                hydrateOnIdle(el);
        }
    }

    function init() {
        var islands = document.querySelectorAll("accent-island");
        for (var i = 0; i < islands.length; i++) {
            scheduleHydration(islands[i]);
        }
    }

    window.AccentIslands = {
        register: function (name, initFn) {
            registry[name] = initFn;
            // Snapshot pending to avoid re-entrancy issues if initFn
            // calls register() during synchronous hydration
            var snapshot = pending.slice();
            var stillPending = [];
            for (var i = 0; i < snapshot.length; i++) {
                if (snapshot[i].dataset.component === name) {
                    scheduleHydration(snapshot[i]);
                } else {
                    stillPending.push(snapshot[i]);
                }
            }
            pending = stillPending;
        }
    };

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", init);
    } else {
        init();
    }
})();
