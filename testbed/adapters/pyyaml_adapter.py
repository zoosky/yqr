#!/usr/bin/env python3
"""PyYAML adapter: the reference for what a document means.

PyYAML has no comment model at all -- not a node attribute, not even a
token -- so it answers `load` and `roundtrip` and reports `unsupported`
for anything about comments. That silence is itself an answer worth
recording: a comment question cannot be settled by citing PyYAML.
"""
import json
import sys

import yaml


def canon(v):
    """A value in a form every adapter can be compared on."""
    if isinstance(v, dict):
        return {str(k): canon(x) for k, x in v.items()}
    if isinstance(v, list):
        return [canon(x) for x in v]
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)


def main():
    req = json.loads(sys.stdin.read())
    op = req["op"]
    if op == "version":
        return {"version": f"PyYAML {yaml.__version__}"}
    src = req["source"]
    try:
        if op == "load":
            return {"result": json.dumps(canon(yaml.safe_load(src)), sort_keys=False)}
        if op == "roundtrip":
            return {"result": yaml.safe_dump(yaml.safe_load(src), default_flow_style=False)}
        if op in ("comments", "delete", "set_comment"):
            return {"unsupported": "PyYAML keeps no comments and has no editing model"}
    except Exception as e:  # noqa: BLE001 - the refusal is the answer
        return {"error": f"{type(e).__name__}: {str(e).splitlines()[0]}"}
    return {"error": f"unknown op {op}"}


print(json.dumps(main()))
