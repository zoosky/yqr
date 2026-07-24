/**
 * Copy-to-clipboard island component
 *
 * Adds a "Copy" button to each <pre> block inside the island.
 * Works with both <pre><code> (markdown) and <pre style="..."> (syntax highlighted).
 * Falls back gracefully if the Clipboard API is unavailable or rejects.
 *
 * Props:
 *   label        - Button text (default: "Copy")
 *   copied_label - Text shown after copying (default: "Copied!")
 */
if (window.AccentIslands) {
window.AccentIslands.register("copy-code", function (el, props) {
    var label = props.label || "Copy";
    var copiedLabel = props.copied_label || "Copied!";
    // Skip diagram pass-through blocks (bug b069): the dev-mode Mermaid
    // loader reads each `pre.diagram-passthrough` via `textContent`, so a
    // "Copy" button injected as the pre's first child would prepend "Copy"
    // to the diagram source and make every diagram fail to parse ("No
    // diagram type detected"). Diagram source is not user-facing code, so
    // it does not need a copy button anyway.
    var pres = el.querySelectorAll("pre:not(.diagram-passthrough)");

    function showCopied(btn) {
        btn.textContent = copiedLabel;
        setTimeout(function () { btn.textContent = label; }, 2000);
    }

    function copyFallback(text, btn) {
        var ta = document.createElement("textarea");
        ta.value = text;
        ta.style.position = "fixed";
        ta.style.opacity = "0";
        document.body.appendChild(ta);
        ta.select();
        try {
            document.execCommand("copy");
            showCopied(btn);
        } catch (e) {
            // Silently fail
        }
        document.body.removeChild(ta);
    }

    for (var i = 0; i < pres.length; i++) {
        (function (pre) {
            if (pre.querySelector(".copy-btn")) return;

            pre.style.position = "relative";

            var btn = document.createElement("button");
            btn.className = "copy-btn";
            btn.textContent = label;
            btn.setAttribute("type", "button");
            btn.setAttribute("aria-label", "Copy code to clipboard");

            btn.addEventListener("click", function () {
                var text = pre.textContent;
                if (navigator.clipboard && navigator.clipboard.writeText) {
                    navigator.clipboard.writeText(text).then(
                        function () { showCopied(btn); },
                        function () { copyFallback(text, btn); }
                    );
                } else {
                    copyFallback(text, btn);
                }
            });

            pre.insertBefore(btn, pre.firstChild);
        })(pres[i]);
    }
});
}
