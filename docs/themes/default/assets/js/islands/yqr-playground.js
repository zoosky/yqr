/**
 * yqr playground island.
 *
 * Hydrates the /playground/ page: loads the wasm-bindgen package built
 * from yqr-wasm (URL arrives in data-props, prefix-safe), wires the
 * filter input and preset chips, and renders three output surfaces:
 *
 *   Document — fidelityRun's output with a byte diff against the input
 *              (common prefix/suffix), plus an untouched-bytes counter.
 *   Values   — eval's output values.
 *   Checks   — validate's diagnostics over the editable document.
 *
 * A filter error renders compiler-style: message plus a caret line under
 * the offending span when the ErrorEnvelope carries one. If the wasm
 * fails to load, the island leaves the server-rendered fallback alone.
 */
(function () {
    "use strict";

    function el(tag, className, text) {
        var node = document.createElement(tag);
        if (className) node.className = className;
        if (text !== undefined) node.textContent = text;
        return node;
    }

    /** Longest common prefix/suffix split of two strings (code units). */
    function byteDiff(before, after) {
        var p = 0;
        var max = Math.min(before.length, after.length);
        while (p < max && before.charCodeAt(p) === after.charCodeAt(p)) p++;
        var s = 0;
        while (
            s < max - p &&
            before.charCodeAt(before.length - 1 - s) ===
                after.charCodeAt(after.length - 1 - s)
        ) s++;
        return {
            prefix: after.slice(0, p),
            changed: after.slice(p, after.length - s),
            suffix: after.slice(after.length - s),
            kept: p + s,
        };
    }

    function utf8Length(text) {
        return new TextEncoder().encode(text).length;
    }

    /** Values render as JSON; strings keep their quotes, like yqr's output. */
    function renderValue(value) {
        return JSON.stringify(value, null, 2);
    }

    function init(root, props) {
        var doc = root.querySelector(".yqr-pg-doc");
        var filter = root.querySelector(".yqr-pg-filter");
        var run = root.querySelector(".yqr-pg-run");
        var presets = root.querySelectorAll(".yqr-pg-preset");
        var docOut = root.querySelector(".yqr-pg-document");
        var valuesOut = root.querySelector(".yqr-pg-values");
        var checksOut = root.querySelector(".yqr-pg-checks");
        var counter = root.querySelector(".yqr-pg-counter");
        var errBox = root.querySelector(".yqr-pg-error");
        var fallback = root.querySelector(".yqr-pg-fallback");
        if (!doc || !filter || !run || !docOut) return;

        var mod = null;

        function showError(envelope, filterText) {
            errBox.hidden = false;
            errBox.textContent = "";
            // The envelope message already carries its category prefix
            // ("parse error: ..."), the same prose the CLI prints.
            errBox.appendChild(el("div", "yqr-pg-error-msg",
                "yqr: " +
                (envelope && envelope.message ? envelope.message : String(envelope))));
            var span = envelope && envelope.span;
            if (span && typeof span.start === "number" && filterText) {
                var caret = " ".repeat(span.start) +
                    "^".repeat(Math.max(1, span.end - span.start));
                var pre = el("pre", "yqr-pg-error-span");
                pre.textContent = filterText + "\n" + caret;
                errBox.appendChild(pre);
            }
        }

        function clearError() {
            errBox.hidden = true;
        }

        /**
         * fidelityRun returns what the CLI prints: for a mutating filter
         * that is the whole edited document, for a read it is the value
         * sliced out of the original bytes. The diff and counter only
         * make sense in the first case; the prefix/suffix overlap tells
         * the two apart (an edited document shares most of its bytes
         * with the input, a sliced value does not).
         */
        function renderDocument(before, after) {
            var diff = byteDiff(before, after);
            var total = utf8Length(before);
            if (after === before) {
                docOut.textContent = after;
                counter.textContent = "All " + total.toLocaleString() +
                    " bytes of your document came back untouched.";
                counter.className = "yqr-pg-counter yqr-pg-counter-clean";
                return;
            }
            if (diff.kept >= before.length / 2) {
                docOut.textContent = "";
                docOut.appendChild(document.createTextNode(diff.prefix));
                if (diff.changed) {
                    docOut.appendChild(el("mark", "yqr-pg-changed", diff.changed));
                }
                docOut.appendChild(document.createTextNode(diff.suffix));
                var keptBytes = utf8Length(diff.prefix) + utf8Length(diff.suffix);
                counter.textContent = keptBytes.toLocaleString() + " of " +
                    total.toLocaleString() +
                    " original bytes untouched — the highlighted bytes are the entire edit.";
                counter.className = "yqr-pg-counter yqr-pg-counter-edited";
            } else {
                docOut.textContent = after;
                counter.textContent = "A read never rewrites your document — " +
                    "these output bytes are sliced straight out of your original.";
                counter.className = "yqr-pg-counter yqr-pg-counter-clean";
            }
        }

        function renderValues(values) {
            valuesOut.textContent = values.length
                ? values.map(renderValue).join("\n")
                : "(no output values)";
        }

        function renderChecks() {
            if (!mod) return;
            var findings;
            try {
                findings = mod.validate(doc.value, true);
            } catch (envelope) {
                checksOut.textContent = "validate failed: " +
                    (envelope && envelope.message ? envelope.message : envelope);
                return;
            }
            checksOut.textContent = "";
            if (!findings.length) {
                checksOut.appendChild(el("div", "yqr-pg-check-ok",
                    "validate --strict: no findings. The document is correct YAML."));
                return;
            }
            findings.forEach(function (f) {
                var line = f.position ? f.position[0] : "?";
                var col = f.position ? f.position[1] : "?";
                var d = el("div", "yqr-pg-check-finding",
                    "error[" + f.code + "]: " + f.message +
                    " --> line " + line + ", column " + col);
                checksOut.appendChild(d);
                if (f.help) {
                    checksOut.appendChild(el("div", "yqr-pg-check-help", "help: " + f.help));
                }
            });
        }

        function execute() {
            if (!mod) return;
            clearError();
            var filterText = filter.value;
            var input = doc.value;
            var after;
            try {
                after = mod.fidelityRun(filterText, input);
            } catch (envelope) {
                showError(envelope, filterText);
                docOut.textContent = input;
                counter.textContent = "";
                valuesOut.textContent = "";
                return;
            }
            renderDocument(input, after);
            try {
                renderValues(mod._eval(filterText, input));
            } catch (envelope) {
                // fidelityRun accepted the program, so eval's refusal
                // means the filter only exists in the byte-preserving
                // pipeline (mutations, comment reads) — unless it is a
                // data-shape refusal (kind "data", e.g. an integer past
                // 2^53), which is worth showing verbatim.
                valuesOut.textContent = (envelope && envelope.kind === "data")
                    ? envelope.message
                    : "(this filter runs in the byte-preserving pipeline " +
                      "only — its output is shown above)";
            }
            renderChecks();
        }

        var pkgBase = props && props.pkg;
        if (!pkgBase) return;
        import(pkgBase + "/yqr_wasm.js")
            .then(function (glue) {
                return glue.default({ module_or_path: pkgBase + "/yqr_wasm_bg.wasm" })
                    .then(function () { return glue; });
            })
            .then(function (glue) {
                mod = glue;
                if (fallback) fallback.hidden = true;
                root.querySelector(".yqr-pg-ui").hidden = false;
                run.addEventListener("click", execute);
                filter.addEventListener("keydown", function (e) {
                    if (e.key === "Enter") execute();
                });
                doc.addEventListener("input", renderChecks);
                presets.forEach(function (chip) {
                    chip.addEventListener("click", function () {
                        filter.value = chip.dataset.filter;
                        execute();
                    });
                });
                execute();
            })
            .catch(function () {
                // wasm unavailable; the server-rendered fallback stays.
            });
    }

    if (window.AccentIslands) {
        window.AccentIslands.register("yqr-playground", init);
    }
})();
