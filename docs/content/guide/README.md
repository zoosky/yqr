---
title: yqr guide
lead: >-
  Where to start: five task-shaped pages covering byte-for-byte reads, surgical edits, validation, enumeration, and arriving from jq.
description: >-
  Practical guides for yqr: what byte-for-byte editing means, editing
  Kubernetes manifests without reformatting them, validating YAML,
  enumerating a mapping without losing the keys, and which jq habits carry
  over.
menu:
  title: Guide
  order: 2
---

# Guide

Short, task-shaped pages. Every command in them was run against a real
build, and the output you see is the output it printed.

- [Byte-for-byte, explained](fidelity) -- what yqr preserves, why the
  identity filter is a useful test, and when you want `--normalize`
  instead.
- [Editing Kubernetes manifests](kubernetes) -- bumping image tags and
  replica counts so the diff is one line.
- [Validating YAML](validate) -- checking a file is still correct after an
  edit, and catching the duplicate keys that silently eat your data.
- [Enumerating mappings](enumerate) -- reporting what each value is about,
  with `to_entries`, when iterating throws the names away.
- [Coming to yqr from jq](from-jq) -- which jq idioms work unchanged, the one
  operator that means something else, and where the two languages stop.

If you are arriving from another tool, [how yqr compares to
yq](../compare/yq) is probably the faster orientation.
