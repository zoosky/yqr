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

/// One block-mapping entry whose value sits on a later line without being
/// indented past its key.
///
/// Both offsets are document-relative at scan time and file-absolute after
/// the caller adds the document's base offset, exactly as [`DuplicateKey`]'s
/// are.
pub(crate) struct UnderIndentedValue {
    /// Byte offset of the entry's key token.
    pub key: usize,
    /// Byte offset of the first byte of the value.
    pub value: usize,
}

/// Every block-mapping entry in `doc` whose value starts on a later line at a
/// column no deeper than its key's, offsets shifted by `base`.
///
/// A block mapping's value, when it does not sit on the key's line, must be
/// indented past the key. noyalib's parser accepts one that is not and hands
/// back a tree in which the under-indented node is the entry's value; other
/// implementations reject the document outright, so nothing downstream of the
/// parse can notice. Hence the green-tree walk, for the same reason the
/// duplicate-key scan exists.
pub(crate) fn under_indented_values(doc: &Document, base: usize) -> Vec<UnderIndentedValue> {
    let mut out = Vec::new();
    scan_indent(doc.syntax(), 0, doc.source(), &mut out);
    for hit in &mut out {
        hit.key += base;
        hit.value += base;
    }
    out
}

/// Depth-first walk: check every entry of every block mapping, then recurse.
fn scan_indent(node: &GreenNode, offset: usize, source: &str, out: &mut Vec<UnderIndentedValue>) {
    if node.kind() == SyntaxKind::BlockMapping {
        let mut child_offset = offset;
        for child in node.children() {
            if let GreenChild::Node(entry) = child
                && entry.kind() == SyntaxKind::MappingEntry
                && let Some(hit) = under_indented_entry(entry, child_offset, source)
            {
                out.push(hit);
            }
            child_offset += child.text_len();
        }
    }
    let mut child_offset = offset;
    for child in node.children() {
        if let GreenChild::Node(inner) = child {
            scan_indent(inner, child_offset, source, out);
        }
        child_offset += child.text_len();
    }
}

/// The finding for one mapping entry, if it has one.
///
/// Returns `None` for every entry whose value is where it belongs, and for
/// the two layouts that look under-indented but are not:
///
/// - **A block sequence at the key's own column** (`on:` / `- push`, the
///   GitHub Actions idiom) — explicitly permitted by the spec, and accepted
///   everywhere. In the tree it is the entry's own `-` token rather than a
///   nested node, so it is recognised by that token.
/// - **A block scalar** (`|` / `>`), whose indentation is set by its own
///   content, so the header may sit at the key's column.
///
/// Entries whose key is not a plain scalar token before the `:` are skipped
/// too — an alias key has no key column to compare against, and an explicit
/// `? key` measures its value against the `?` under a different rule.
fn under_indented_entry(
    entry: &GreenNode,
    offset: usize,
    source: &str,
) -> Option<UnderIndentedValue> {
    let (key_offset, value_offset, value_is_block_seq) = entry_key_and_value(entry, offset)?;
    let (key_line, key_column) = position_in(source, key_offset);
    let (value_line, value_column) = position_in(source, value_offset);
    if value_line == key_line || value_column > key_column {
        return None;
    }
    // A block sequence is allowed to share its key's column, and only that
    // column — one shallower and it would belong to an outer collection.
    if value_is_block_seq && value_column == key_column {
        return None;
    }
    Some(UnderIndentedValue {
        key: key_offset,
        value: value_offset,
    })
}

/// The entry's key offset, the offset of the first byte of its value, and
/// whether that value is a block sequence.
///
/// The value is the first non-trivia child after the `:` indicator. An entry
/// with no value (`a:` followed by a sibling) yields `None`, as does one whose
/// key is not a plain scalar token.
fn entry_key_and_value(entry: &GreenNode, offset: usize) -> Option<(usize, usize, bool)> {
    let mut child_offset = offset;
    let mut key: Option<usize> = None;
    let mut past_colon = false;
    for child in entry.children() {
        match child {
            GreenChild::Token { kind, .. } => {
                if *kind == SyntaxKind::QuestionIndicator {
                    // An explicit key (`? a` / `: b`) puts the `:` on a line of
                    // its own, so its value is measured against the `?`, not
                    // against the key token. A different rule, and not one this
                    // scan claims to know.
                    return None;
                }
                if *kind == SyntaxKind::ColonIndicator {
                    key?;
                    past_colon = true;
                } else if past_colon {
                    match kind {
                        SyntaxKind::Whitespace | SyntaxKind::Newline | SyntaxKind::Comment => {}
                        SyntaxKind::LiteralScalar | SyntaxKind::FoldedScalar => return None,
                        SyntaxKind::DashIndicator => {
                            return Some((key?, child_offset, true));
                        }
                        _ => return Some((key?, child_offset, false)),
                    }
                } else if key.is_none() && is_scalar_token(*kind) {
                    key = Some(child_offset);
                }
            }
            GreenChild::Node(inner) => {
                if past_colon {
                    let is_block_seq = inner.kind() == SyntaxKind::BlockSequence;
                    return Some((key?, child_offset, is_block_seq));
                }
            }
        }
        child_offset += child.text_len();
    }
    None
}

/// 1-based `(line, column)` of `byte` within `source`.
///
/// A local walk rather than `render::position_of`: the scan works in
/// document-relative offsets, and lines and columns are the same whether they
/// are counted from the start of the document or the start of the file, since
/// a document always begins at a line boundary.
fn position_in(source: &str, byte: usize) -> (usize, usize) {
    let byte = byte.min(source.len());
    let start = source[..byte].rfind('\n').map_or(0, |i| i + 1);
    let line = source[..byte].bytes().filter(|&b| b == b'\n').count() + 1;
    (line, source[start..byte].chars().count() + 1)
}
