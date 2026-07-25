/**
 * Contact form island component (accent-contact-island plugin)
 *
 * Progressively enhances the contact form rendered by the
 * accent-contact plugin. Intercepts form submit, validates
 * client-side, sends via fetch(), and shows inline feedback.
 *
 * When JavaScript is disabled, the form falls back to the
 * standard POST + redirect flow handled by accent-contact.
 *
 * Props: none (all configuration comes from the form itself)
 */
if (window.AccentIslands) {
window.AccentIslands.register("contact-form", function (el) {
    var form = el.querySelector("form");
    if (!form) return;

    // Hide server-rendered error messages (from ?error= query params).
    // When the island hydrates, it handles errors inline instead.
    var serverErrors = document.querySelectorAll(".contact-server-error");
    for (var i = 0; i < serverErrors.length; i++) {
        serverErrors[i].style.display = "none";
    }

    // Create a status container for inline messages
    var status = document.createElement("div");
    status.className = "contact-island-status";
    status.setAttribute("role", "alert");
    status.setAttribute("aria-live", "polite");
    form.parentNode.insertBefore(status, form.nextSibling);

    function clearStatus() {
        while (status.firstChild) {
            status.removeChild(status.firstChild);
        }
    }

    function showMessage(message, className) {
        clearStatus();
        var div = document.createElement("div");
        div.className = "alert " + className;
        var p = document.createElement("p");
        p.textContent = message;
        div.appendChild(p);
        status.appendChild(div);
    }

    function showError(message) {
        showMessage(message, "alert-error");
    }

    function showSuccess(message) {
        showMessage(message, "alert-success");
        form.style.display = "none";
    }

    // Client-side validation using browser's native HTML5 validation
    // (input[type=email], required attributes) plus custom checks.
    function validate() {
        // Use the browser's built-in constraint validation first.
        // This covers required fields and email format via input[type=email].
        if (!form.checkValidity()) {
            // Find the first invalid field and show its validation message
            var invalid = form.querySelector(":invalid");
            if (invalid) {
                var label = invalid.getAttribute("name") || "field";
                showError(invalid.validationMessage || "Please fill in the " + label + " field.");
            }
            return false;
        }
        return true;
    }

    // Parse error type from redirect URL query string
    function parseError(url) {
        var match = url.match(/[?&]error=([^&]*)/);
        if (!match) return null;
        return decodeURIComponent(match[1]);
    }

    // Map error codes to user-facing messages
    var errorMessages = {
        csrf: "Security token expired. Please reload the page and try again.",
        name: "Please enter your name.",
        email: "Please enter a valid email address.",
        message: "Please enter a message.",
        smtp: "Unable to send your message. Please try again later or email us directly."
    };

    var submitLabel = form.querySelector("button[type='submit']");
    var originalLabel = submitLabel ? submitLabel.textContent : "Send Message";

    function setSubmitting(submitting) {
        if (!submitLabel) return;
        submitLabel.disabled = submitting;
        submitLabel.textContent = submitting ? "Sending..." : originalLabel;
    }

    form.addEventListener("submit", function (e) {
        e.preventDefault();
        clearStatus();

        if (!validate()) return;

        setSubmitting(true);

        var formData = new FormData(form);
        var body = [];
        formData.forEach(function (value, key) {
            body.push(encodeURIComponent(key) + "=" + encodeURIComponent(value));
        });

        fetch(form.action, {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.join("&"),
            redirect: "follow"
        }).then(function (response) {
            var finalUrl = response.url || "";

            // Check if we were redirected to the success page
            if (finalUrl.indexOf("/contact-sent") !== -1) {
                showSuccess("Thank you! Your message has been sent.");
                return;
            }

            // Check for error in the redirect URL
            var errorCode = parseError(finalUrl);
            if (errorCode) {
                var msg = errorMessages[errorCode] ||
                    "Something went wrong. Please check your input and try again.";
                showError(msg);
                return;
            }

            // No recognizable redirect target -- assume something unexpected
            showError("Something went wrong. Please try again later.");
        }).catch(function () {
            showError("Network error. Please check your connection and try again.");
        }).finally(function () {
            setSubmitting(false);
        });
    });
});
}
