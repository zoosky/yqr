//! JSON Schema validation of parsed documents, with source spans.
//!
//! This is the second half of the editing-loop verdict: after
//! [`check_str`](super::check_str) answers *is this correct YAML?*, the
//! checks here answer *is it the YAML this system accepts?* — against a
//! JSON Schema 2020-12 document the caller provides.
//!
//! The differentiator over kubeconform-class tools is the position. A
//! violation arrives from the validator as a JSON-pointer instance path
//! with no source location; its typed segments map one-to-one onto the
//! fidelity engine's [`Path`], and [`FidelityEngine::resolve`] maps that
//! to the byte span of the node in the original source. A violation
//! therefore renders exactly like every other finding: `--> file:line:col`,
//! source window, caret.
//!
//! Validation runs on the JSON data model, so the schema never sees YAML
//! anchors, tags or spellings — only the value tree. The one value the
//! parse boundary can produce that JSON cannot represent is a non-finite
//! float; such a document is reported (`Y203`) rather than silently
//! coerced, because coercion would change what the schema sees.
//!
//! No build of yqr resolves external `$ref`s: the validator is compiled
//! without network or filesystem resolution, and a schema that needs
//! either fails to compile (`Y202`).

// Feature f040: the --schema flag on the validate subcommand.

use crate::fidelity::{FidelityEngine, Path, PathSeg, Resolved};
use crate::value::Value;

use super::{Code, Diagnostic, render};

/// A compiled JSON Schema 2020-12 document, ready to validate against.
///
/// Compiled once per `validate` run and applied to every input; the
/// wrapped validator type stays private so the schema engine remains a
/// swappable implementation detail, the same posture the YAML engine has.
pub struct Schema {
    validator: jsonschema::Validator,
}

impl std::fmt::Debug for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schema").finish_non_exhaustive()
    }
}

impl Schema {
    /// Compile `source` — a schema document written in YAML or JSON —
    /// into a validator.
    ///
    /// # Errors
    ///
    /// Returns one ready-to-render diagnostic (`Y202`) when the schema is
    /// unusable: not valid YAML, not a single document, outside the JSON
    /// data model, or not a valid JSON Schema 2020-12 document (an
    /// external `$ref` is in that class, since nothing resolves it).
    pub fn compile(source: &str) -> std::result::Result<Self, Diagnostic> {
        let docs = match ::noyalib::cst::parse_stream_with_config(
            source,
            &crate::fidelity::cst_config(),
        ) {
            Ok(docs) => docs,
            Err(err) => {
                // Reuse the syntax diagnostic wholesale — position, merge
                // conflict detection and collision precision included —
                // under the schema code, so a broken schema file is as
                // actionable as a broken input.
                let mut diag = super::syntax_diagnostic(&err, source);
                diag.code = Code::SchemaUnusable;
                diag.message = format!("schema is not valid YAML: {}", diag.message);
                return Err(diag);
            }
        };
        if docs.len() != 1 {
            return Err(schema_unusable(format!(
                "schema must be a single document, but the file holds {}",
                docs.len()
            )));
        }
        drop(docs);
        let engine = crate::fidelity::open(source)
            .map_err(|err| schema_unusable(format!("schema is not readable: {err}")))?;
        let value = engine
            .value(0)
            .map_err(|err| schema_unusable(format!("schema is not readable: {err}")))?;
        let json = to_json(&value, &Path::root()).map_err(|(path, reason)| {
            schema_unusable(format!(
                "schema {} at {}",
                reason.describe(),
                pointer_display(&path)
            ))
        })?;
        let validator = jsonschema::draft202012::new(&json).map_err(|err| {
            schema_unusable(format!(
                "schema does not compile as JSON Schema 2020-12: {err}"
            ))
        })?;
        Ok(Schema { validator })
    }
}

/// Build a position-less `Y202` diagnostic about the schema file.
fn schema_unusable(message: String) -> Diagnostic {
    Diagnostic {
        code: Code::SchemaUnusable,
        message,
        position: None,
        note: None,
        help: Some("the schema is a single JSON Schema 2020-12 document, in YAML or JSON".into()),
    }
}

/// Validate every document of `source` against `schema` and return the
/// findings, in document order then violation order.
///
/// An input that does not parse returns no findings: the caller's default
/// checks already reported the `Y001`, and there is no tree to validate.
/// A document outside the JSON data model (a non-finite float) yields one
/// `Y203` and skips schema validation for that document only.
#[must_use]
pub fn check_schema(source: &str, schema: &Schema) -> Vec<Diagnostic> {
    let Ok(engine) = crate::fidelity::open(source) else {
        return Vec::new();
    };
    let starts = super::document_starts(source);
    let mut findings = Vec::new();
    for doc in 0..engine.doc_count() {
        let Ok(value) = engine.value(doc) else {
            continue;
        };
        let json = match to_json(&value, &Path::root()) {
            Ok(json) => json,
            Err((path, reason)) => {
                findings.push(json_model_diagnostic(
                    engine.as_ref(),
                    doc,
                    &path,
                    reason,
                    source,
                    &starts,
                ));
                continue;
            }
        };
        for err in schema.validator.iter_errors(&json) {
            findings.push(violation_diagnostic(
                engine.as_ref(),
                doc,
                &err,
                source,
                &starts,
            ));
        }
    }
    findings
}

/// Build the `Y201` finding for one schema violation.
///
/// The position is the resolved span of the instance path — or, when the
/// pointed-at node has no bytes of its own (merge- or alias-expanded
/// content), of the nearest enclosing node that does. The note always
/// carries the exact instance path, so nothing is lost when the caret
/// lands on an ancestor.
fn violation_diagnostic(
    engine: &dyn FidelityEngine,
    doc: usize,
    err: &jsonschema::ValidationError<'_>,
    source: &str,
    starts: &[usize],
) -> Diagnostic {
    let path = instance_path(err.instance_path());
    let anchor = anchor_byte(engine, doc, &path);
    Diagnostic {
        code: Code::SchemaViolation,
        message: err.to_string(),
        position: Some(render::position_of(source, anchor)),
        note: Some(located_note(&path, source, starts, anchor)),
        help: None,
    }
}

/// Build the `Y203` finding for a document the JSON data model cannot
/// represent.
fn json_model_diagnostic(
    engine: &dyn FidelityEngine,
    doc: usize,
    path: &Path,
    reason: JsonGap,
    source: &str,
    starts: &[usize],
) -> Diagnostic {
    let anchor = anchor_byte(engine, doc, path);
    Diagnostic {
        code: Code::OutsideJsonModel,
        message: format!("document {}", reason.describe()),
        position: Some(render::position_of(source, anchor)),
        note: Some(located_note(path, source, starts, anchor)),
        help: Some(
            "JSON Schema validates the JSON data model; this document was not \
             validated against the schema"
                .into(),
        ),
    }
}

/// The note line shared by `Y201` and `Y203`: the instance path, plus the
/// document in a multi-document stream.
fn located_note(path: &Path, source: &str, starts: &[usize], anchor: usize) -> String {
    let mut note = if path.is_root() {
        "at the document root".to_string()
    } else {
        format!("at instance path {}", pointer_display(path))
    };
    if let Some(doc_note) = super::document_note(source, starts, anchor) {
        note.push_str(", ");
        note.push_str(&doc_note);
    }
    note
}

/// The first byte of the source span the path resolves to.
///
/// Segments are popped until the path resolves to original bytes: a
/// merge- or alias-expanded node has none of its own, and pointing at its
/// nearest enclosing node beats pointing nowhere. The root always
/// resolves (its span is the document slice), so the loop terminates with
/// a position for every finding.
fn anchor_byte(engine: &dyn FidelityEngine, doc: usize, path: &Path) -> usize {
    let mut path = path.clone();
    loop {
        if let Ok(Resolved::Found { span, .. }) = engine.resolve(doc, &path) {
            return span.start;
        }
        match parent(&path) {
            Some(p) => path = p,
            None => return engine.doc_span(doc).map_or(0, |span| span.start),
        }
    }
}

/// The path one segment shorter, or `None` at the root.
fn parent(path: &Path) -> Option<Path> {
    let segments = path.segments();
    let (_, init) = segments.split_last()?;
    let mut parent = Path::root();
    for seg in init {
        parent = parent.child(seg.clone());
    }
    Some(parent)
}

/// Convert the validator's typed instance location into a fidelity
/// [`Path`].
///
/// The segments arrive already decoded (RFC 6901 `~0`/`~1` unescaping is
/// the validator's), so a property name maps straight onto
/// [`PathSeg::Key`].
fn instance_path(location: &jsonschema::paths::Location) -> Path {
    let mut path = Path::root();
    for segment in location.segments() {
        path = path.child(match segment {
            jsonschema::paths::LocationSegment::Property(name) => PathSeg::Key(name.into_owned()),
            jsonschema::paths::LocationSegment::Index(index) => PathSeg::Index(index),
        });
    }
    path
}

/// Render a fidelity path as an RFC 6901 JSON pointer for the note line.
fn pointer_display(path: &Path) -> String {
    let mut out = String::new();
    for seg in path.segments() {
        out.push('/');
        match seg {
            PathSeg::Key(key) => out.push_str(&key.replace('~', "~0").replace('/', "~1")),
            PathSeg::Index(index) => out.push_str(&index.to_string()),
        }
    }
    out
}

/// What keeps a value out of the JSON data model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonGap {
    /// A float with no JSON form (`.nan`, `.inf`).
    NonFiniteNumber,
    /// A mapping key that is not a string. The parse boundary produces
    /// only string keys, so this arm guards library callers, not files.
    NonStringKey,
}

impl JsonGap {
    /// Human wording for the diagnostic message.
    const fn describe(self) -> &'static str {
        match self {
            JsonGap::NonFiniteNumber => "contains a non-finite number, which JSON cannot represent",
            JsonGap::NonStringKey => {
                "contains a non-string mapping key, which JSON cannot represent"
            }
        }
    }
}

/// Convert a yqr [`Value`] into the JSON instance the validator reads.
///
/// Fails — with the path of the offending node — instead of coercing: a
/// schema validating a silently altered value would return a verdict
/// about a document that does not exist.
fn to_json(value: &Value, path: &Path) -> std::result::Result<serde_json::Value, (Path, JsonGap)> {
    Ok(match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::from(*i),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| (path.clone(), JsonGap::NonFiniteNumber))?,
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Sequence(items) => serde_json::Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, item)| to_json(item, &path.child(PathSeg::Index(i))))
                .collect::<std::result::Result<_, _>>()?,
        ),
        Value::Mapping(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, val) in map {
                let Value::String(key) = key else {
                    return Err((path.clone(), JsonGap::NonStringKey));
                };
                let child = path.child(PathSeg::Key(key.clone()));
                out.insert(key.clone(), to_json(val, &child)?);
            }
            serde_json::Value::Object(out)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(src: &str) -> Schema {
        Schema::compile(src).expect("schema compiles")
    }

    #[test]
    fn conforming_input_has_no_findings() {
        let s = schema(
            "type: object\nproperties:\n  replicas:\n    type: integer\nrequired: [replicas]\n",
        );
        assert!(check_schema("replicas: 3\n", &s).is_empty());
    }

    #[test]
    fn type_violation_points_at_the_offending_scalar() {
        let s = schema(
            "type: object\nproperties:\n  spec:\n    type: object\n    properties:\n      replicas:\n        type: integer\n",
        );
        let source = "name: app\nspec:\n  replicas: \"three\"\n";
        let findings = check_schema(source, &s);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        let d = &findings[0];
        assert_eq!(d.code, Code::SchemaViolation);
        assert_eq!(d.position, Some((3, 13)), "points at the scalar's bytes");
        assert_eq!(d.note.as_deref(), Some("at instance path /spec/replicas"));
        assert!(d.message.contains("integer"), "message: {}", d.message);
    }

    #[test]
    fn missing_required_property_points_at_the_parent_mapping() {
        let s =
            schema("type: object\nproperties:\n  spec:\n    type: object\n    required: [image]\n");
        let findings = check_schema("spec:\n  replicas: 3\n", &s);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        let d = &findings[0];
        // The instance path names the object that lacks the property; its
        // span starts on its first entry's line.
        assert_eq!(d.position, Some((2, 1)));
        assert_eq!(d.note.as_deref(), Some("at instance path /spec"));
    }

    #[test]
    fn root_violation_reports_the_document_root() {
        let s = schema("type: array\n");
        let findings = check_schema("a: 1\n", &s);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].position, Some((1, 1)));
        assert_eq!(findings[0].note.as_deref(), Some("at the document root"));
    }

    #[test]
    fn stream_positions_are_absolute_and_name_the_document() {
        let s = schema("type: object\nproperties:\n  n:\n    type: integer\n");
        let findings = check_schema("n: 1\n---\nn: x\n", &s);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        let d = &findings[0];
        assert_eq!(d.position, Some((3, 4)));
        assert_eq!(
            d.note.as_deref(),
            Some("at instance path /n, in document 2 (starting at line 2)")
        );
    }

    #[test]
    fn every_document_of_a_stream_is_validated() {
        let s = schema("type: object\n");
        let findings = check_schema("- 1\n---\n- 2\n", &s);
        assert_eq!(findings.len(), 2, "findings: {findings:?}");
    }

    #[test]
    fn alias_expanded_violation_points_at_the_anchor_definition() {
        // The engine resolves alias content at the anchor's definition —
        // the same place a write through the alias lands — so the caret
        // points at the actual offending bytes, and the note carries the
        // instance path that reached them.
        let s = schema(
            "type: object\nproperties:\n  use:\n    type: object\n    properties:\n      k:\n        type: integer\n",
        );
        let source = "base: &b\n  k: x\nuse: *b\n";
        let findings = check_schema(source, &s);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        let d = &findings[0];
        assert_eq!(d.note.as_deref(), Some("at instance path /use/k"));
        let position = d.position.expect("every violation has a position");
        assert_eq!(position.0, 2, "anchors on the definition's `k: x` line");
    }

    #[test]
    fn merge_expanded_violation_falls_back_to_bytes_that_exist() {
        // A merge-produced entry has no bytes of its own, and the
        // wrong-node guard keeps every enclosing mapping Synthetic too —
        // the pop-to-ancestor rule ends at the document root. The note
        // still names the exact instance path.
        let s = schema(
            "type: object\nproperties:\n  use:\n    type: object\n    properties:\n      k:\n        type: integer\n",
        );
        let source = "defaults: &d\n  k: x\nuse:\n  <<: *d\n";
        let findings = check_schema(source, &s);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        let d = &findings[0];
        assert_eq!(d.note.as_deref(), Some("at instance path /use/k"));
        assert_eq!(d.position, Some((1, 1)), "falls back to the root span");
    }

    #[test]
    fn key_holding_a_slash_is_escaped_in_the_note_and_resolved() {
        let s = schema("type: object\nproperties:\n  a/b:\n    type: integer\n");
        let findings = check_schema("a/b: x\n", &s);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        assert_eq!(findings[0].position, Some((1, 6)));
        assert_eq!(findings[0].note.as_deref(), Some("at instance path /a~1b"));
    }

    #[test]
    fn non_finite_float_is_a_y203_that_skips_the_document() {
        let s = schema("type: object\n");
        // Document 1 leaves the JSON model; document 2 violates the
        // schema — the first is reported as Y203, the second as Y201.
        let findings = check_schema("x: .nan\n---\n- 1\n", &s);
        assert_eq!(findings.len(), 2, "findings: {findings:?}");
        assert_eq!(findings[0].code, Code::OutsideJsonModel);
        assert_eq!(findings[0].position, Some((1, 4)));
        assert!(
            findings[0]
                .note
                .as_deref()
                .is_some_and(|n| n.contains("/x")),
            "note: {:?}",
            findings[0].note
        );
        assert_eq!(findings[1].code, Code::SchemaViolation);
    }

    #[test]
    fn unparseable_input_yields_no_schema_findings() {
        let s = schema("type: object\n");
        assert!(check_schema("a: [1,\n", &s).is_empty());
    }

    #[test]
    fn schema_with_a_yaml_syntax_error_is_a_y202_with_position() {
        let err = Schema::compile("a: [1,\n").expect_err("must not compile");
        assert_eq!(err.code, Code::SchemaUnusable);
        assert!(err.message.starts_with("schema is not valid YAML:"));
        assert!(err.position.is_some());
    }

    #[test]
    fn multi_document_schema_is_a_y202() {
        let err = Schema::compile("type: object\n---\ntype: array\n").expect_err("refused");
        assert_eq!(err.code, Code::SchemaUnusable);
        assert!(err.message.contains("single document"), "{}", err.message);
    }

    #[test]
    fn invalid_schema_document_is_a_y202() {
        // `type: 5` is not a valid 2020-12 schema.
        let err = Schema::compile("type: 5\n").expect_err("refused");
        assert_eq!(err.code, Code::SchemaUnusable);
        assert!(
            err.message.contains("JSON Schema 2020-12"),
            "{}",
            err.message
        );
    }

    #[test]
    fn external_ref_fails_to_compile() {
        // No build of yqr resolves one: the validator is compiled with
        // network and filesystem resolution off.
        let err =
            Schema::compile("$ref: \"https://example.com/schema.json\"\n").expect_err("refused");
        assert_eq!(err.code, Code::SchemaUnusable);
    }

    #[test]
    fn json_schema_file_reads_as_yaml() {
        // JSON is a subset of YAML, so a .json schema file needs no
        // special path.
        let s = schema("{\"type\": \"object\", \"required\": [\"a\"]}");
        assert_eq!(check_schema("b: 1\n", &s).len(), 1);
    }

    #[test]
    fn pointer_display_escapes_rfc6901() {
        let path = Path::root()
            .child(PathSeg::Key("a/b".into()))
            .child(PathSeg::Key("c~d".into()))
            .child(PathSeg::Index(2));
        assert_eq!(pointer_display(&path), "/a~1b/c~0d/2");
    }
}
