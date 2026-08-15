/**
 * Accent CMS Client-Side Search (Features 092, 102).
 *
 * Dual-backend search library that checks `data-search-backend` on
 * `.acms-search-form` elements:
 *
 * - "docfind": loads the DocFind WASM engine for FST fuzzy matching
 * - "simple" (default): fetches a JSON index with substring matching
 *
 * Attaches to all elements with the `.acms-search-form` class.
 */
(function () {
  "use strict";

  var DEBOUNCE_MS = 200;

  // ===========================================================================
  // Shared rendering helpers
  // ===========================================================================

  // Escape text destined for innerHTML. Covers the four characters that can
  // change parsing in element content and in a double-quoted attribute value,
  // so one helper serves both the result URLs and the indexed title/snippet
  // text. Everything the renderer interpolates must pass through here: indexed
  // text is page content, and page content is authored, not trusted markup.
  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/"/g, "&quot;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function escapeRegex(s) {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  // Collect the [start, end) ranges of every query-token match in `text`,
  // sorted by start and with overlaps merged, so a character is highlighted at
  // most once no matter how many tokens cover it.
  function matchRanges(text, queryTokens) {
    var ranges = [];
    for (var i = 0; i < queryTokens.length; i++) {
      if (!queryTokens[i]) continue;
      var re = new RegExp(escapeRegex(queryTokens[i]), "gi");
      var m;
      while ((m = re.exec(text)) !== null) {
        if (m[0].length === 0) {
          re.lastIndex++;
          continue;
        }
        ranges.push([m.index, m.index + m[0].length]);
      }
    }
    ranges.sort(function (a, b) {
      return a[0] - b[0] || a[1] - b[1];
    });
    var merged = [];
    for (var j = 0; j < ranges.length; j++) {
      var last = merged.length > 0 ? merged[merged.length - 1] : null;
      if (last && ranges[j][0] <= last[1]) {
        if (ranges[j][1] > last[1]) last[1] = ranges[j][1];
      } else {
        merged.push([ranges[j][0], ranges[j][1]]);
      }
    }
    return merged;
  }

  // Build the highlighted HTML for one indexed field: escape every segment of
  // the source text and wrap the matched ones in <mark>, so the marks are the
  // only markup in the result.
  //
  // Matching runs over the *raw* text and escaping happens per segment, rather
  // than escaping up front and matching the escaped string: escaping first
  // would let a token match inside an entity this function had just produced
  // (a query of "amp" hitting the "amp" of "&amp;"), splitting it into visible
  // garbage. It also keeps the marks out of the search space, so tokens like
  // "mark" or "class" cannot match markup the previous iteration inserted.
  function highlightText(text, queryTokens, maxLen) {
    if (!text) return "";
    var snippet = text.length > maxLen ? text.substring(0, maxLen) + "..." : text;
    var ranges = matchRanges(snippet, queryTokens);
    var html = "";
    var pos = 0;
    for (var i = 0; i < ranges.length; i++) {
      html += escapeHtml(snippet.substring(pos, ranges[i][0]));
      html +=
        '<mark class="acms-search-highlight">' +
        escapeHtml(snippet.substring(ranges[i][0], ranges[i][1])) +
        "</mark>";
      pos = ranges[i][1];
    }
    return html + escapeHtml(snippet.substring(pos));
  }

  function renderResults(container, results, queryTokens) {
    if (results.length === 0) {
      container.innerHTML =
        '<div class="acms-search-empty">No results found.</div>';
      container.hidden = false;
      return;
    }

    var html = "";
    for (var i = 0; i < results.length; i++) {
      var r = results[i];
      var title = highlightText(r.title, queryTokens, 200);
      var snippet = highlightText(r.snippet, queryTokens, 150);
      html +=
        '<a class="acms-search-result" href="' +
        escapeHtml(r.url) +
        '" role="option"' +
        (i === 0 ? ' aria-selected="true"' : "") +
        ">" +
        '<span class="acms-search-result-title">' +
        title +
        "</span>" +
        '<span class="acms-search-result-snippet">' +
        snippet +
        "</span>" +
        "</a>";
    }

    container.innerHTML = html;
    container.hidden = false;
  }

  function tokenize(text) {
    if (!text) return [];
    return text
      .toLowerCase()
      .replace(/[^\w\s]/g, " ")
      .split(/\s+/)
      .filter(function (w) {
        return w.length >= 2;
      });
  }

  // ===========================================================================
  // Version scoping (Feature f225)
  //
  // Versioned content lives at `/<prefix>/<id>/...`, so a page's version is the
  // path segment after its versioning root. The index ships a `versionRoots`
  // map; we keep only results whose version matches the reader's current
  // version (or each root's default when the reader is not inside a version).
  // Pages not under any versioned root are always shown.
  // ===========================================================================

  // Resolve { prefix, version } for a URL, or null if not under a root.
  // `version` is null when the URL is under a root but has no valid version
  // segment. A `"/"` root is a catch-all (every URL is under it), mirroring
  // `url_under_root` in the server's versioning config.
  function versionRootOf(url, roots) {
    for (var i = 0; i < roots.length; i++) {
      var r = roots[i];
      var under =
        r.prefix === "/" || url === r.prefix || url.indexOf(r.prefix + "/") === 0;
      if (under) {
        // Path relative to the root, without a leading slash; the version is its
        // first segment. Works for `/docs` (rest starts after `/docs/`) and for
        // the `/` catch-all root (rest is the whole path minus its leading `/`).
        var rest = url.slice(r.prefix.length);
        if (rest.charAt(0) === "/") rest = rest.slice(1);
        var seg = rest.split("/")[0];
        return { prefix: r.prefix, version: r.ids.indexOf(seg) !== -1 ? seg : null };
      }
    }
    return null;
  }

  // Map of root prefix -> version to scope to: each root's default, overridden
  // by the reader's current version when the current page is inside that root.
  function versionScope(roots, currentPath) {
    var scope = {};
    for (var i = 0; i < roots.length; i++) {
      scope[roots[i].prefix] = roots[i].default;
    }
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

  // ===========================================================================
  // Simple Search backend (Feature 092)
  // ===========================================================================

  var legacyIndex = null;
  var legacyLoading = false;
  var legacyCallbacks = [];

  function loadLegacyIndex(cb) {
    if (legacyIndex) {
      cb(legacyIndex);
      return;
    }
    legacyCallbacks.push(cb);
    if (legacyLoading) return;
    legacyLoading = true;

    var xhr = new XMLHttpRequest();
    xhr.open("GET", "/yqr/_search/index.json", true);
    xhr.onreadystatechange = function () {
      if (xhr.readyState !== 4) return;
      if (xhr.status === 200) {
        try {
          legacyIndex = JSON.parse(xhr.responseText);
        } catch (e) {
          legacyIndex = { pages: [], weights: { t: 3, c: 1, g: 2, l: 1.5 } };
        }
      } else {
        legacyIndex = { pages: [], weights: { t: 3, c: 1, g: 2, l: 1.5 } };
      }
      // Tokenize the short fields once, up front, so no query re-tokenizes the
      // corpus (b081). Cheap: title/lead/tags only -- content is pre-tokenized.
      prepareLegacyIndex(legacyIndex);
      for (var i = 0; i < legacyCallbacks.length; i++) {
        legacyCallbacks[i](legacyIndex);
      }
      legacyCallbacks = [];
    };
    xhr.send();
  }

  function countMatches(tokens, queryTokens) {
    var count = 0;
    for (var i = 0; i < queryTokens.length; i++) {
      for (var j = 0; j < tokens.length; j++) {
        if (tokens[j].indexOf(queryTokens[i]) !== -1) {
          count++;
          break;
        }
      }
    }
    return count;
  }

  // Precompute per-page token arrays exactly once, right after the index loads,
  // so queries never re-tokenize the corpus (b081). The content tokens (`ct`)
  // arrive already tokenized and deduplicated from the server; only the short
  // title/lead/tags fields are tokenized here, a single time. Without this the
  // scorer re-tokenized every page's content on every keystroke -- O(corpus)
  // per query -- which froze the main thread on large indexes.
  function prepareLegacyIndex(data) {
    if (!data || data._prepared) return;
    var pages = data.pages || [];
    for (var i = 0; i < pages.length; i++) {
      var page = pages[i];
      page._tt = tokenize(page.t);
      page._lt = tokenize(page.l || "");
      var tagTokens = [];
      if (page.g) {
        for (var j = 0; j < page.g.length; j++) {
          var t = tokenize(page.g[j]);
          for (var k = 0; k < t.length; k++) {
            tagTokens.push(t[k]);
          }
        }
      }
      page._gt = tagTokens;
      if (!page.ct) page.ct = [];
    }
    data._prepared = true;
  }

  function scorePage(page, queryTokens, weights) {
    return (
      countMatches(page._tt, queryTokens) * weights.t +
      countMatches(page.ct, queryTokens) * weights.c +
      countMatches(page._lt, queryTokens) * weights.l +
      countMatches(page._gt, queryTokens) * weights.g
    );
  }

  function legacySearch(data, query, limit) {
    var queryTokens = tokenize(query);
    if (queryTokens.length === 0) return [];

    // Scope results to the reader's current version (f225).
    var roots = data.versionRoots || null;
    var scope = roots ? versionScope(roots, location.pathname) : null;

    var results = [];
    for (var i = 0; i < data.pages.length; i++) {
      var page = data.pages[i];
      if (roots && !inVersionScope(page.u, roots, scope)) continue;
      var score = scorePage(page, queryTokens, data.weights);
      if (score > 0) {
        results.push({
          url: page.u,
          title: page.t,
          // New indexes always set `l`; `page.c` is a fallback for a cached
          // old-format index that still carries the raw content blob (b081).
          snippet: page.l || page.c || "",
          score: score,
        });
      }
    }

    results.sort(function (a, b) {
      return b.score - a.score;
    });

    return results.slice(0, limit);
  }

  function initLegacySearch(input, resultsContainer, limit) {
    var debounceTimer = null;

    function doSearch() {
      var query = input.value.trim();
      if (query.length < 2) {
        resultsContainer.innerHTML = "";
        resultsContainer.hidden = true;
        activeIndex = -1;
        return;
      }

      loadLegacyIndex(function (data) {
        var results = legacySearch(data, query, limit);
        var queryTokens = tokenize(query);
        renderResults(resultsContainer, results, queryTokens);
      });
    }

    input.addEventListener("input", function () {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(doSearch, DEBOUNCE_MS);
    });

    return { doSearch: doSearch };
  }

  // ===========================================================================
  // DocFind search backend (Feature 102)
  // ===========================================================================

  var docfindModule = null;
  var docfindLoading = false;
  var docfindFailed = false;
  var docfindCallbacks = [];

  // Load the DocFind WASM module, invoking `cb(module)` on success. If the glue
  // or WASM cannot be loaded (e.g. the committed placeholder artifacts), `cb(null)`
  // is invoked instead so the caller can fall back to Simple Search rather than
  // silently returning no results. A failed load is remembered so subsequent
  // queries fall back immediately instead of re-attempting the import per keystroke.
  function loadDocFind(cb) {
    if (docfindModule) {
      cb(docfindModule);
      return;
    }
    if (docfindFailed) {
      cb(null);
      return;
    }
    docfindCallbacks.push(cb);
    if (docfindLoading) return;
    docfindLoading = true;

    import("/yqr/_search/docfind.js")
      .then(function (mod) {
        return mod.default("/yqr/_search/docfind_bg.wasm").then(function () {
          docfindModule = mod;
          var cbs = docfindCallbacks;
          docfindCallbacks = [];
          for (var i = 0; i < cbs.length; i++) {
            cbs[i](docfindModule);
          }
        });
      })
      .catch(function () {
        docfindFailed = true;
        docfindLoading = false;
        var cbs = docfindCallbacks;
        docfindCallbacks = [];
        for (var i = 0; i < cbs.length; i++) {
          cbs[i](null);
        }
      });
  }

  function docfindSearch(mod, query, limit) {
    try {
      var raw = mod.search(query, limit);
      var results = [];
      for (var i = 0; i < raw.length; i++) {
        var r = raw[i];
        results.push({
          url: r.href,
          title: r.title,
          snippet: r.body ? r.body.substring(0, 200) : "",
          score: r.score || 1,
        });
      }
      return results;
    } catch (_) {
      return [];
    }
  }

  function initDocFindSearch(form, input, resultsContainer, limit) {
    var debounceTimer = null;

    function doSearch() {
      var query = input.value.trim();
      if (query.length < 2) {
        resultsContainer.innerHTML = "";
        resultsContainer.hidden = true;
        activeIndex = -1;
        return;
      }

      loadDocFind(function (mod) {
        var queryTokens = tokenize(query);
        if (!mod) {
          // DocFind glue/WASM unavailable -- fall back to the Simple Search index
          // so search keeps working instead of silently returning nothing.
          loadLegacyIndex(function (data) {
            var results = legacySearch(data, query, limit);
            renderResults(resultsContainer, results, queryTokens);
          });
          return;
        }
        var results = docfindSearch(mod, query, limit);
        renderResults(resultsContainer, results, queryTokens);
      });
    }

    input.addEventListener("input", function () {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(doSearch, DEBOUNCE_MS);
    });

    // Check stale indicator via HEAD request
    checkDocFindStale(form);

    return { doSearch: doSearch };
  }

  function checkDocFindStale(form) {
    var xhr = new XMLHttpRequest();
    xhr.open("HEAD", "/yqr/_search/docfind_bg.wasm", true);
    xhr.onreadystatechange = function () {
      if (xhr.readyState !== 4) return;
      var stale = xhr.getResponseHeader("X-Search-Index-Stale");
      if (stale === "true") {
        form.classList.add("acms-search-stale");
      } else {
        form.classList.remove("acms-search-stale");
      }
    };
    xhr.send();
  }

  // ===========================================================================
  // Form initialization (shared keyboard navigation and outside-click)
  // ===========================================================================

  function initForm(form) {
    var input = form.querySelector(".acms-search-input");
    var resultsContainer = form.querySelector(".acms-search-results");
    if (!input || !resultsContainer) return;

    var limit = parseInt(form.getAttribute("data-limit") || "10", 10);
    var backend = form.getAttribute("data-search-backend") || "simple";
    var activeIndex = -1;
    if (backend === "docfind") {
      initDocFindSearch(form, input, resultsContainer, limit);
    } else {
      initLegacySearch(input, resultsContainer, limit);
    }

    input.addEventListener("focus", function () {
      // hasChildNodes() rather than reading innerHTML: same truthiness, but it
      // does not serialize the whole result list to a string on every focus.
      if (input.value.trim().length >= 2 && resultsContainer.hasChildNodes()) {
        resultsContainer.hidden = false;
      }
    });

    input.addEventListener("keydown", function (e) {
      var items = resultsContainer.querySelectorAll(".acms-search-result");
      if (items.length === 0) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        activeIndex = Math.min(activeIndex + 1, items.length - 1);
        updateActive(items);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        activeIndex = Math.max(activeIndex - 1, 0);
        updateActive(items);
      } else if (e.key === "Enter") {
        // Follow the highlighted result. Before any ArrowUp/ArrowDown,
        // activeIndex is -1 but the first result is rendered with
        // aria-selected="true" (see renderResults), so Enter should follow
        // it. items.length > 0 is guaranteed by the early return above.
        e.preventDefault();
        var idx = activeIndex >= 0 ? activeIndex : 0;
        items[idx].click();
      } else if (e.key === "Escape") {
        resultsContainer.hidden = true;
        activeIndex = -1;
      }
    });

    function updateActive(items) {
      for (var i = 0; i < items.length; i++) {
        items[i].setAttribute(
          "aria-selected",
          i === activeIndex ? "true" : "false"
        );
        if (i === activeIndex) {
          items[i].scrollIntoView({ block: "nearest" });
        }
      }
    }

    document.addEventListener("click", function (e) {
      if (!form.contains(e.target)) {
        resultsContainer.hidden = true;
        activeIndex = -1;
      }
    });
  }

  function init() {
    var forms = document.querySelectorAll(".acms-search-form");
    for (var i = 0; i < forms.length; i++) {
      initForm(forms[i]);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
