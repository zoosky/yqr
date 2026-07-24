/**
 * Interactive search island component
 *
 * Enhances a server-rendered search form with client-side filtering
 * against a JSON search index. The fallback form works without JS.
 *
 * Props:
 *   endpoint    - URL to the search index JSON (default: "/_search/index.json")
 *   max_results - Maximum number of results to show (default: 10)
 *   min_chars   - Minimum characters before searching (default: 2)
 *   debounce    - Debounce delay in ms (default: 300)
 */
if (window.AccentIslands) {
window.AccentIslands.register("search", function (el, props) {
    var endpoint = props.endpoint || "/_search/index.json";
    var maxResults = props.max_results || 10;
    var minChars = props.min_chars || 2;
    var debounceMs = props.debounce || 300;

    var input = el.querySelector("input[type=search], input[type=text]");
    if (!input) return;

    var results = document.createElement("div");
    results.className = "island-search-results";
    results.setAttribute("role", "listbox");
    results.setAttribute("aria-label", "Search results");
    el.appendChild(results);

    var indexCache = null;
    var timeout;

    function fetchIndex(callback) {
        if (indexCache) {
            callback(indexCache);
            return;
        }
        var xhr = new XMLHttpRequest();
        xhr.open("GET", endpoint, true);
        xhr.onreadystatechange = function () {
            if (xhr.readyState === 4 && xhr.status === 200) {
                try {
                    indexCache = JSON.parse(xhr.responseText);
                    callback(indexCache);
                } catch (e) {
                    // Failed to parse search index
                }
            }
        };
        xhr.send();
    }

    function isSafeUrl(url) {
        if (!url) return false;
        // Allow only relative URLs and http(s) schemes
        if (url.charAt(0) === "/" || url.charAt(0) === ".") return true;
        var lower = url.toLowerCase();
        return lower.indexOf("http://") === 0 || lower.indexOf("https://") === 0;
    }

    // Normalize index entries: the Accent CMS search index uses short field
    // names (u, t, l, ct); an external index may instead use full names for the
    // url/title/excerpt fields, which the accessors below fall back to. The
    // pre-tokenized, deduplicated content field (`ct`, b081) is Accent-specific,
    // so an external index without it is matched on title and excerpt only;
    // `l` is a short display snippet.
    function pageUrl(p) { return p.url || p.u || ""; }
    function pageTitle(p) { return p.title || p.t || ""; }
    function pageExcerpt(p) { return p.excerpt || p.description || p.l || ""; }
    function pageContentTokens(p) { return p.ct || []; }

    // Tokenize a query the same way the index tokenizes content (lowercase,
    // split on any run outside [A-Za-z0-9_], drop tokens < 2 chars) so a
    // punctuated single-word query like "hot-reload" matches the pre-tokenized
    // `ct` content tokens instead of silently missing (b081).
    function tokenize(text) {
        if (!text) return [];
        return text.toLowerCase().replace(/[^\w\s]/g, " ").split(/\s+/).filter(function (w) {
            return w.length >= 2;
        });
    }

    // Version scoping (f225): keep only results in the reader's current version.
    // Versioned content lives at `/<prefix>/<id>/...`; the index ships a
    // `versionRoots` map and a page's version is the segment after its root.
    function versionRootOf(url, roots) {
        for (var i = 0; i < roots.length; i++) {
            var r = roots[i];
            // A "/" root is a catch-all (every URL is under it), mirroring the
            // server's url_under_root; the version is the first path segment
            // after the root prefix.
            var under = r.prefix === "/" || url === r.prefix || url.indexOf(r.prefix + "/") === 0;
            if (under) {
                var rest = url.slice(r.prefix.length);
                if (rest.charAt(0) === "/") rest = rest.slice(1);
                var seg = rest.split("/")[0];
                return { prefix: r.prefix, version: r.ids.indexOf(seg) !== -1 ? seg : null };
            }
        }
        return null;
    }
    function versionScope(roots, currentPath) {
        var scope = {};
        for (var i = 0; i < roots.length; i++) scope[roots[i].prefix] = roots[i].default;
        var cur = versionRootOf(currentPath, roots);
        if (cur && cur.version) scope[cur.prefix] = cur.version;
        return scope;
    }
    function inVersionScope(url, roots, scope) {
        if (!roots || roots.length === 0) return true;
        var rv = versionRootOf(url, roots);
        if (!rv || !rv.version) return true; // not version-specific -> always show
        return rv.version === scope[rv.prefix];
    }

    // Lowercased, cached "searchable text" for a page: its title, snippet, and
    // pre-tokenized content tokens joined once. Built lazily on first use so a
    // query matches with a native substring scan instead of re-deriving text
    // per keystroke (the per-term split means token order is irrelevant).
    function searchableText(p) {
        if (p._st === undefined) {
            p._st = (pageTitle(p) + " " + pageExcerpt(p) + " " +
                pageContentTokens(p).join(" ")).toLowerCase();
        }
        return p._st;
    }

    function renderResults(hits) {
        results.innerHTML = "";
        if (hits.length === 0) {
            var p = document.createElement("p");
            p.className = "no-results";
            p.textContent = "No results found";
            results.appendChild(p);
            return;
        }
        for (var k = 0; k < hits.length; k++) {
            var h = hits[k];
            var url = pageUrl(h);
            if (!isSafeUrl(url)) continue;
            var a = document.createElement("a");
            a.className = "search-hit";
            a.href = url;
            a.setAttribute("role", "option");
            var strong = document.createElement("strong");
            strong.textContent = pageTitle(h);
            a.appendChild(strong);
            var exc = pageExcerpt(h);
            if (exc) {
                var span = document.createElement("span");
                span.textContent = exc;
                a.appendChild(span);
            }
            results.appendChild(a);
        }
    }

    function search(query) {
        var terms = tokenize(query);
        if (terms.length === 0) {
            results.innerHTML = "";
            return;
        }
        fetchIndex(function (index) {
            var pages = index.pages || index;
            var roots = index.versionRoots || null;
            var scope = roots ? versionScope(roots, location.pathname) : null;
            var hits = [];
            for (var i = 0; i < pages.length && hits.length < maxResults; i++) {
                var page = pages[i];
                if (roots && !inVersionScope(pageUrl(page), roots, scope)) continue;
                var text = searchableText(page);
                var match = true;
                for (var j = 0; j < terms.length; j++) {
                    if (text.indexOf(terms[j]) === -1) {
                        match = false;
                        break;
                    }
                }
                if (match) hits.push(page);
            }
            renderResults(hits);
        });
    }

    input.addEventListener("input", function () {
        clearTimeout(timeout);
        var q = input.value.trim();
        if (q.length < minChars) {
            results.innerHTML = "";
            return;
        }
        timeout = setTimeout(function () { search(q); }, debounceMs);
    });

    // Close results when clicking outside
    document.addEventListener("click", function (e) {
        if (!el.contains(e.target)) {
            results.innerHTML = "";
        }
    });
});
}
