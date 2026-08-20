//! Round-trip fidelity harness for yqr's YAML engine.
//!
//! Pins research r002 as runnable tests: `noyalib`'s CST round-trips
//! byte-for-byte. For every fidelity dimension, it parses the input and
//! re-emits it, then checks that the output is byte-for-byte identical to the
//! input -- the a001 north-star property that `yqr '.' f` must equal `cat f`.
//!
//! Run the human-readable comparison matrix:
//!
//! ```text
//! cargo test --test fidelity -- --nocapture fidelity_matrix
//! ```
//!
//! The harness is backend-agnostic (the [`Backend`] trait); noyalib is the only
//! engine yqr ships today, but another YAML crate can be compared by
//! implementing [`Backend`] for it and registering it in `backends()`.

/// A single fidelity dimension drawn from the b001 reproduction corpus.
struct Case {
    /// Stable identifier used in the matrix and in per-case assertions.
    name: &'static str,
    /// Byte-exact input. CRLF, BOM, and trailing whitespace are spelled with
    /// explicit escapes so the literal is unambiguous.
    input: &'static str,
}

/// Outcome of one parse -> emit round trip, relative to the input bytes.
#[derive(Debug, PartialEq, Eq)]
enum Fidelity {
    /// Output is byte-for-byte identical to the input (the goal).
    Identical,
    /// Parsed and re-emitted, but the bytes changed (formatting was lost).
    Differs,
    /// The backend refused or failed to parse the input.
    Error,
}

/// A YAML library that can attempt a lossless parse -> emit round trip.
///
/// Implementors expose just enough to be compared: a display name and a single
/// `round_trip` that loads the document and serializes it back. Everything else
/// (classification, reporting) is shared.
trait Backend {
    /// Short label shown in the comparison matrix.
    fn name(&self) -> &'static str;

    /// Parse `input` and re-emit it. `Ok(text)` is the re-emitted document;
    /// `Err(msg)` means the backend could not parse the input.
    fn round_trip(&self, input: &str) -> Result<String, String>;

    /// Classify the round trip against the original bytes.
    fn classify(&self, input: &str) -> Fidelity {
        match self.round_trip(input) {
            Ok(out) if out == input => Fidelity::Identical,
            Ok(_) => Fidelity::Differs,
            Err(_) => Fidelity::Error,
        }
    }
}

/// `noyalib`'s lossless CST tooling API: `cst::parse_document -> Display`.
///
/// yqr's sole YAML engine (research r002); this harness pins its byte-for-byte
/// round-trip property across every formatting dimension.
struct NoyalibCst;

impl Backend for NoyalibCst {
    fn name(&self) -> &'static str {
        "noyalib (CST)"
    }

    fn round_trip(&self, input: &str) -> Result<String, String> {
        noyalib::cst::parse_document(input)
            .map(|doc| doc.to_string())
            .map_err(|e| e.to_string())
    }
}

/// All backends under comparison. yqr now has a single engine, noyalib's CST.
fn backends() -> Vec<Box<dyn Backend>> {
    vec![Box::new(NoyalibCst)]
}

/// The corpus: one case per formatting dimension that a faithful round trip
/// must preserve. Mirrors the b001 reproduction set.
const CORPUS: &[Case] = &[
    Case {
        name: "comments",
        input: concat!(
            "# Top-level header comment\n",
            "name: my-app   # inline comment on a scalar\n",
            "\n",
            "# Section: replicas\n",
            "replicas: 3\n",
            "\n",
            "config:\n",
            "  # nested comment\n",
            "  debug: true\n",
            "  level: info\n",
        ),
    },
    Case {
        name: "blank-lines",
        input: "a: 1\n\nb: 2\n\n\nc: 3\n",
    },
    Case {
        name: "indent-width",
        input: concat!(
            "root:\n",
            "    child:\n",
            "        leaf: value\n",
            "    sibling: other\n",
        ),
    },
    Case {
        name: "quote-style",
        input: concat!(
            "bare: hello\n",
            "single: 'hello world'\n",
            "double: \"hello world\"\n",
            "forced_string: \"123\"\n",
            "special: 'it''s a test'\n",
        ),
    },
    Case {
        name: "block-scalars",
        input: concat!(
            "literal: |\n",
            "  line one\n",
            "  line two\n",
            "folded: >\n",
            "  this is\n",
            "  folded text\n",
        ),
    },
    Case {
        name: "numbers",
        input: concat!(
            "replicas: 3\n",
            "ratio: 1.0\n",
            "zip: 007\n",
            "big_id: 12345678901234567\n",
            "port: 8080\n",
            "neg: -5\n",
        ),
    },
    Case {
        name: "flow-style",
        input: concat!(
            "flow_map: {a: 1, b: 2}\n",
            "flow_seq: [1, 2, 3]\n",
            "nested: {list: [x, y], n: 1}\n",
        ),
    },
    // A flow collection wrapped over several lines, with the closing indicator
    // at the parent key's column. Every implementation but noyalib accepted
    // this; noyalib refused to parse it at all (bug b011) until 0.0.25, so the
    // round-trip property had no case for the shape.
    Case {
        name: "wrapped-flow",
        input: concat!(
            "ports: [\n",
            "  80,\n",
            "  443,\n",
            "]\n",
            "opts: {\n",
            "  retries: 3,\n",
            "}\n",
        ),
    },
    Case {
        name: "key-order",
        input: "zebra: 1\napple: 2\nmango: 3\n",
    },
    Case {
        name: "anchors-merge",
        input: concat!(
            "defaults: &defaults\n",
            "  timeout: 30\n",
            "  retries: 3\n",
            "service:\n",
            "  <<: *defaults\n",
            "  name: web\n",
        ),
    },
    Case {
        name: "crlf",
        input: "a: 1\r\nb: 2\r\n",
    },
    Case {
        // A UTF-8 BOM followed by multiple top-level nodes. noyalib 0.0.8
        // rejected this (r002 5.3); fixed in 0.0.12 (noyalib#123), it now
        // round-trips byte-for-byte.
        name: "bom-multinode",
        input: "\u{FEFF}a: 1\nb: 2\n",
    },
    Case {
        name: "trailing-ws",
        input: "a: 1   \nb: 2\t\n",
    },
    Case {
        name: "multi-document",
        input: "---\na: 1\n---\nb: 2\n",
    },
    Case {
        name: "k8s-manifest",
        input: concat!(
            "# Production deployment\n",
            "apiVersion: apps/v1\n",
            "kind: Deployment\n",
            "metadata:\n",
            "  name: web        # the web frontend\n",
            "  labels:\n",
            "    app: web\n",
            "spec:\n",
            "  replicas: 3      # scale here\n",
            "\n",
            "  template:\n",
            "    spec:\n",
            "      containers:\n",
            "        - name: web\n",
            "          image: nginx:1.25   # pin the tag\n",
        ),
    },
];

/// Prints a backend-by-dimension fidelity matrix. Never fails -- this is the
/// reproduction tool; run with `-- --nocapture` to see it.
#[test]
fn fidelity_matrix() {
    let backends = backends();

    print!("{:<16}", "dimension");
    for b in &backends {
        print!(" | {:<18}", b.name());
    }
    println!();
    println!("{}", "-".repeat(16 + backends.len() * 21));

    for case in CORPUS {
        print!("{:<16}", case.name);
        for b in &backends {
            let label = match b.classify(case.input) {
                Fidelity::Identical => "IDENTICAL",
                Fidelity::Differs => "DIFFERS",
                Fidelity::Error => "ERROR",
            };
            print!(" | {label:<18}");
        }
        println!();
    }
}

/// Pins research r002: `noyalib`'s CST reproduces the source byte-for-byte for
/// **every** corpus dimension. The BOM-with-multiple-nodes parse bug that failed
/// on 0.0.8 (r002 §5.3) was fixed upstream in 0.0.12 (noyalib#123), so it now
/// round-trips like the rest. If any dimension regresses, this test fails and
/// flags that the r002 evaluation needs updating.
#[test]
fn noyalib_cst_round_trip_is_faithful() {
    let backend = NoyalibCst;
    for case in CORPUS {
        assert_eq!(
            backend.classify(case.input),
            Fidelity::Identical,
            "noyalib fidelity changed for `{}` -- update r002 if intentional",
            case.name,
        );
    }
}
