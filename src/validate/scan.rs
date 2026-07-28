//! Duplicate mapping-key detection over noyalib's green tree.
//!
//! The value layer cannot see duplicates (they are resolved last-wins
//! before any caller looks), and its duplicate-key policy both stops at
//! the first offence and exempts `<<` merge keys. Walking the lossless
//! green tree instead finds **every** duplicate — merge keys included —
//! each with the byte offsets of both occurrences, which the diagnostics
//! turn into real source positions.

// Feature f012: strict-mode duplicate-key scan.

use ::noyalib::cst::{Document, GreenChild, GreenNode, SyntaxKind};
use std::collections::HashMap;

/// One duplicated mapping key: the decoded key string and the byte offsets
/// (document-relative at scan time, file-absolute after the caller adds
/// the document's base offset) of the first and the repeated occurrence.
pub(crate) struct DuplicateKey {
    /// The key, decoded (quotes stripped, escapes resolved).
    pub key: String,
    /// Byte offset of the first occurrence's key token.
    pub first: usize,
    /// Byte offset of the repeated occurrence's key token.
    pub second: usize,
}

/// Every duplicated key in `doc`, offsets shifted by `base` (the
/// document's byte offset within the file).
pub(crate) fn duplicate_keys(doc: &Document, base: usize) -> Vec<DuplicateKey> {
    let mut out = Vec::new();
    scan(doc.syntax(), 0, doc.source(), &mut out);
    for dup in &mut out {
        dup.first += base;
        dup.second += base;
    }
    out
}

/// Whether `kind` is a scalar token that can serve as a mapping key.
fn is_scalar_token(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::PlainScalar
            | SyntaxKind::SingleQuotedScalar
            | SyntaxKind::DoubleQuotedScalar
            | SyntaxKind::LiteralScalar
            | SyntaxKind::FoldedScalar
    )
}

/// Depth-first walk: record duplicates for every mapping node, then recurse
/// into child nodes with running byte offsets.
fn scan(node: &GreenNode, offset: usize, source: &str, out: &mut Vec<DuplicateKey>) {
    match node.kind() {
        SyntaxKind::BlockMapping => {
            report_duplicates(block_mapping_keys(node, offset, source), out);
        }
        SyntaxKind::FlowMapping => {
            report_duplicates(flow_mapping_keys(node, offset, source), out);
        }
        _ => {}
    }
    let mut child_offset = offset;
    for child in node.children() {
        if let GreenChild::Node(inner) = child {
            scan(inner, child_offset, source, out);
        }
        child_offset += child.text_len();
    }
}

/// Fold a mapping's `(decoded key, offset)` list into duplicate records.
fn report_duplicates(keys: Vec<(String, usize)>, out: &mut Vec<DuplicateKey>) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (key, offset) in keys {
        match seen.get(&key) {
            Some(&first) => out.push(DuplicateKey {
                key,
                first,
                second: offset,
            }),
            None => {
                seen.insert(key, offset);
            }
        }
    }
}

/// The `(decoded key, byte offset)` of every entry key in a block mapping.
///
/// A block mapping's children are `MappingEntry` nodes (plus trivia); the
/// entry's key is the first scalar token before its `:` indicator. Entries
/// whose key is not a scalar token (alias keys, explicit complex keys) are
/// skipped — they cannot be compared as strings.
fn block_mapping_keys(node: &GreenNode, offset: usize, source: &str) -> Vec<(String, usize)> {
    let mut keys = Vec::new();
    let mut child_offset = offset;
    for child in node.children() {
        if let GreenChild::Node(entry) = child
            && entry.kind() == SyntaxKind::MappingEntry
            && let Some(key) = entry_key(entry, child_offset, source)
        {
            keys.push(key);
        }
        child_offset += child.text_len();
    }
    keys
}

/// The first scalar token before the `:` indicator of a mapping entry.
fn entry_key(entry: &GreenNode, offset: usize, source: &str) -> Option<(String, usize)> {
    let mut child_offset = offset;
    for child in entry.children() {
        if let GreenChild::Token { kind, .. } = child {
            if *kind == SyntaxKind::ColonIndicator {
                return None;
            }
            if is_scalar_token(*kind) {
                let raw = child.token_text(source, child_offset)?;
                return Some((decode_key(*kind, raw), child_offset));
            }
        }
        child_offset += child.text_len();
    }
    None
}

/// The `(decoded key, byte offset)` of every key in a flow mapping.
///
/// Flow content is kept flat: the children are the braces, separators, and
/// the key/value tokens in source order. A scalar token immediately (up to
/// trivia) followed by a `:` indicator is a key; everything else — values,
/// nested collections, aliases — resets the candidate.
fn flow_mapping_keys(node: &GreenNode, offset: usize, source: &str) -> Vec<(String, usize)> {
    let mut keys = Vec::new();
    let mut candidate: Option<(String, usize)> = None;
    let mut child_offset = offset;
    for child in node.children() {
        match child {
            GreenChild::Token { kind, .. } if is_scalar_token(*kind) => {
                candidate = child
                    .token_text(source, child_offset)
                    .map(|raw| (decode_key(*kind, raw), child_offset));
            }
            GreenChild::Token { kind, .. } if *kind == SyntaxKind::ColonIndicator => {
                if let Some(key) = candidate.take() {
                    keys.push(key);
                }
            }
            GreenChild::Node(_) => candidate = None,
            GreenChild::Token { .. } => {}
        }
        child_offset += child.text_len();
    }
    keys
}

/// Decode a key token's raw source text to the string the YAML key denotes.
///
/// Plain (and block-scalar) keys are taken verbatim; quoted keys lose
/// their quotes and resolve their escapes, so `a`, `'a'`, and `"a"` all
/// compare equal — they are the same YAML string key.
fn decode_key(kind: SyntaxKind, raw: &str) -> String {
    match kind {
        SyntaxKind::SingleQuotedScalar => strip_quotes(raw).replace("''", "'"),
        SyntaxKind::DoubleQuotedScalar => unescape_double(strip_quotes(raw)),
        _ => raw.to_string(),
    }
}

/// Drop one leading and one trailing quote character, when present.
fn strip_quotes(raw: &str) -> &str {
    let raw = raw.strip_prefix(['\'', '"']).unwrap_or(raw);
    raw.strip_suffix(['\'', '"']).unwrap_or(raw)
}

/// Resolve the common double-quote escapes; unknown escapes are kept
/// verbatim (a mismatch there can only make two keys compare unequal,
/// never falsely equal).
fn unescape_double(inner: &str) -> String {
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
