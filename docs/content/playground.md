---
# Traceability: Marketing k003; the bindings are Feature f049. The preset
# filters and the sample document are the measured table in the k003 spec:
# change one here, re-verify there.
title: Playground
lead: >-
  The same engine as the CLI, compiled to WebAssembly, running in your
  browser. Type a filter, edit the document, and watch exactly which bytes
  an edit touches.
template: playground
menu:
  order: 2
---

## What you are looking at

This page runs yqr itself — not a lookalike. The Rust engine behind the
CLI is compiled to a WebAssembly module, and every run above calls the
same entry points `yqr '.server.port = 9090' config.yaml` calls on your
machine.

Everything happens locally. Your document never leaves the page, which is
the point: yqr exists for configuration files, and configuration files are
exactly what you don't paste into someone else's server.

## What to try

- **Run a read.** Any read — the first preset, or `.` — reports every byte
  untouched. That is not a rendering choice: the engine slices your
  original bytes rather than re-serializing a parse tree.
- **Run an edit.** `.server.port = 9090` highlights the only bytes that
  changed. The comment at the end of that line, the odd spacing before it,
  and every other line survive byte-for-byte.
- **Paste your own YAML.** The document pane is editable. Bring the most
  comment-laden, strangely indented file you have; that is the file the
  fidelity guarantee is for.
- **Break the filter.** The last preset is a parse error. The offending
  spot is underlined under the filter box, the way the CLI prints it —
  errors are structured data here, not strings.
- **Break the document.** Duplicate a key and watch the checks pane: the
  findings are the same diagnostics `yqr validate --strict` prints, with
  code, line and column.

## Where to go next

- The [fidelity guide](/guide/fidelity/) explains how byte-preserving
  editing works and where its edges are.
- The [demo script](/demo/) walks the full surface — including the parts a
  browser page can't show, like exit codes and stdin — against files you
  can inspect.
- [From jq](/guide/from-jq/) maps the filter language onto what you
  already know.

## For embedders

The module this page loads is the published `yqr-wasm` bindings crate:
`eval` for reads, `fidelityRun` for byte-preserving execution, `validate`
for diagnostics, and a typed edit API for programs. If you are building an
editor, the playground is a working integration to crib from — view
source, or start at the bindings crate in the repository.
