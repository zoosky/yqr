#!/usr/bin/env python3
"""ruamel.yaml adapter: the closest peer to what yqr does.

A round-trip loader that keeps comments, quoting and key order, so it is
the reference for the questions PyYAML cannot answer. Where it differs
from yqr the difference is a design decision worth naming, not a bug in
either.
"""
import io
import json
import sys

import ruamel.yaml
from ruamel.yaml import YAML


def rt():
    y = YAML()
    y.preserve_quotes = True
    return y


def canon(v):
    if hasattr(v, "items"):
        return {str(k): canon(x) for k, x in v.items()}
    if isinstance(v, list):
        return [canon(x) for x in v]
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)


def walk(node, path):
    for seg in path:
        node = node[seg]
    return node


def comments_of(doc, path):
    """What ruamel attaches to the node at `path`, and to its parent entry."""
    out = {}
    parent = walk(doc, path[:-1]) if len(path) > 1 else doc
    key = path[-1]
    ca = getattr(parent, "ca", None)
    if ca is not None:
        item = ca.items.get(key)
        if item:
            # (post_key, pre_key, post_value, pre_value) tokens
            for label, tok in zip(("post_key", "pre_key", "post_value", "pre_value"), item):
                if tok is None:
                    continue
                toks = tok if isinstance(tok, list) else [tok]
                for t in toks:
                    if t is not None and getattr(t, "value", None):
                        out.setdefault(label, []).append(t.value.strip())
    return out


def main():
    req = json.loads(sys.stdin.read())
    op = req["op"]
    if op == "version":
        return {"version": f"ruamel.yaml {ruamel.yaml.__version__}"}
    src = req["source"]
    path = req.get("path")
    try:
        if op == "load":
            return {"result": json.dumps(canon(rt().load(src)), sort_keys=False)}
        if op == "roundtrip":
            buf = io.StringIO()
            y = rt()
            y.dump(y.load(src), buf)
            return {"result": buf.getvalue()}
        if op == "comments":
            doc = rt().load(src)
            return {"result": json.dumps(comments_of(doc, path), sort_keys=True)}
        if op == "delete":
            doc = rt().load(src)
            parent = walk(doc, path[:-1]) if len(path) > 1 else doc
            del parent[path[-1]]
            buf = io.StringIO()
            rt().dump(doc, buf)
            return {"result": buf.getvalue()}
    except Exception as e:  # noqa: BLE001
        return {"error": f"{type(e).__name__}: {str(e).splitlines()[0]}"}
    return {"error": f"unknown op {op}"}


print(json.dumps(main()))
