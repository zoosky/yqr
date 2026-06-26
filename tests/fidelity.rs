//! Round-trip fidelity harness across YAML backend libraries.
//!
//! Reproduces, as runnable tests, the assessment behind bug b001 (the shipped
//! `rust-yaml` engine reformats YAML on round trip) and research r002
//! (`noyalib`'s CST round-trips byte-for-byte). For every backend and every
//! fidelity dimension, it parses the input and re-emits it, then checks whether
//! the output is byte-for-byte identical to the input -- the a001 north-star
//! property that `yqr '.' f` must equal `cat f`.
//!
//! Run the human-readable comparison matrix (rust-yaml only, the default):
//!
//! ```text
//! cargo test --test fidelity -- --nocapture fidelity_matrix
//! ```
//!
//! Include the experimental `noyalib` CST backend (gated, off by default so the
//! standard test build stays minimal):
//!
//! ```text
//! cargo test --test fidelity --features backend-noyalib -- --nocapture
//! ```
//!
//! Adding another backend (e.g. a different YAML crate) is a two-line change:
//! implement [`Backend`] for it and register it in `backends()`.

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

/// The engine yqr ships today: `rust-yaml` 1.1.0 via `text -> Value -> dump_str`.
///
/// This is the lossy semantic round trip documented in bug b001; it is expected
/// to reformat most inputs.
struct RustYaml;

impl Backend for RustYaml {
    fn name(&self) -> &'static str {
        "rust-yaml (Value)"
    }

    fn round_trip(&self, input: &str) -> Result<String, String> {
        let yaml = rust_yaml::Yaml::new();
        let value = yaml.load_str(input).map_err(|e| e.to_string())?;
        yaml.dump_str(&value).map_err(|e| e.to_string())
    }
}

/// `noyalib`'s lossless CST tooling API: `cst::parse_document -> Display`.
///
/// Experimental comparison backend (research r002), gated behind the
/// `backend-noyalib` feature so the default test build does not pull it in.
#[cfg(feature = "backend-noyalib")]
struct NoyalibCst;

#[cfg(feature = "backend-noyalib")]
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

/// All backends under comparison. `rust-yaml` is always present; `noyalib` is
/// added only when the `backend-noyalib` feature is enabled.
fn backends() -> Vec<Box<dyn Backend>> {
    #[allow(unused_mut)]
    let mut v: Vec<Box<dyn Backend>> = vec![Box::new(RustYaml)];
    #[cfg(feature = "backend-noyalib")]
    v.push(Box::new(NoyalibCst));
    v
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
        // rejects this (r002 5.3); rust-yaml folds the BOM into the first key.
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

/// Look up a corpus case by name (panics if the name is unknown -- used only by
/// tests over the constant corpus).
fn case(name: &str) -> &'static Case {
    CORPUS
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("unknown corpus case `{name}`"))
}

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

/// Characterization test pinning bug b001: the shipped `rust-yaml` engine does
/// NOT preserve formatting on these dimensions. If a future `rust-yaml` bump
/// fixes one of them this assertion will fail, prompting a revisit of b001/a001.
#[test]
fn rust_yaml_round_trip_is_lossy() {
    let backend = RustYaml;
    for name in [
        "comments",
        "blank-lines",
        "indent-width",
        "quote-style",
        "block-scalars",
        "flow-style",
    ] {
        assert_eq!(
            backend.classify(case(name).input),
            Fidelity::Differs,
            "rust-yaml unexpectedly preserved `{name}`; b001 may be (partly) fixed -- revisit the spec",
        );
    }
}

/// Pins research r002: `noyalib`'s CST reproduces the source byte-for-byte for
/// every corpus dimension EXCEPT the known BOM-with-multiple-nodes parse bug,
/// which is expected to error in 0.0.8. A change in either direction (a newly
/// failing dimension, or the BOM bug being fixed) will fail this test and flag
/// that the r002 evaluation needs updating.
#[cfg(feature = "backend-noyalib")]
#[test]
fn noyalib_cst_round_trip_is_faithful() {
    let backend = NoyalibCst;
    for case in CORPUS {
        let expected = if case.name == "bom-multinode" {
            Fidelity::Error
        } else {
            Fidelity::Identical
        };
        assert_eq!(
            backend.classify(case.input),
            expected,
            "noyalib fidelity changed for `{}` -- update r002 if intentional",
            case.name,
        );
    }
}
