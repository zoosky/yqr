#!/usr/bin/env python3
"""Run the reference testbed and write (or check) the recorded answers.

    python3 testbed/run.py            # rewrite testbed/answers.md
    python3 testbed/run.py --check    # fail if the answers have drifted

The point is not that yqr agrees with these libraries. It is that when a
design question comes up -- whose comment is this, is that document one
anybody else reads, what should an emptied block become -- the answer is
measured against implementations people actually run, and the measurement
is dated, versioned and reproducible instead of recalled.

See specs/implementation/yqr-m007-reference-testbed.md.
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADAPTERS = HERE / "adapters"

# Ordered: the three that answer "what does this mean", then the two that
# also answer "where does this comment go".
PEERS = [
    ("PyYAML", [sys.executable, str(ADAPTERS / "pyyaml_adapter.py")]),
    ("Psych", ["ruby", str(ADAPTERS / "psych_adapter.rb")]),
    ("js-yaml", ["bun", "run", str(ADAPTERS / "jsyaml_adapter.ts")]),
    ("go-yaml", None),  # a built binary, filled in by build_go
    ("ruamel", [sys.executable, str(ADAPTERS / "ruamel_adapter.py")]),
]


def build_go(tmp: Path) -> list[str] | None:
    """Build the go-yaml adapter once rather than `go run` per call."""
    if shutil.which("go") is None:
        return None
    out = tmp / "goyaml"
    r = subprocess.run(
        ["go", "build", "-o", str(out), "."],
        cwd=ADAPTERS / "goyaml",
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"go-yaml adapter did not build:\n{r.stderr}", file=sys.stderr)
        return None
    return [str(out)]


def ask(cmd: list[str], req: dict) -> dict:
    """One question to one implementation."""
    try:
        r = subprocess.run(
            cmd, input=json.dumps(req), capture_output=True, text=True, timeout=60
        )
    except FileNotFoundError:
        return {"missing": f"{cmd[0]} is not installed"}
    except subprocess.TimeoutExpired:
        return {"error": "timed out"}
    line = r.stdout.strip().splitlines()
    if not line:
        return {"error": (r.stderr.strip().splitlines() or ["no output"])[-1]}
    try:
        return json.loads(line[-1])
    except json.JSONDecodeError:
        return {"error": line[-1]}


def visible(text: str) -> str:
    """The exact bytes, on one line, without markdown eating any of them."""
    return (
        text.replace("\\", "\\\\")
        .replace("\r", "\\r")
        .replace("\n", "\\n")
        .replace("`", "\u2019")
        .replace("|", "\\|")
    )


def render(answer: dict) -> str:
    """One cell of the report."""
    if "missing" in answer:
        return f"_{answer['missing']}_"
    if "unsupported" in answer:
        return f"_unsupported: {answer['unsupported']}_"
    if "error" in answer:
        return f"**refused** `{visible(answer['error'])}`"
    return "`" + visible(answer["result"]) + "`"


def collect(peers, cases) -> dict:
    """Every answer, keyed so a comparison can be made per implementation."""
    out: dict = {"versions": {}, "answers": {}}
    for name, cmd in peers:
        v = ask(cmd, {"op": "version"})
        out["versions"][name] = v.get("version", v.get("missing", "?"))
    for case in cases:
        for op in case["ops"]:
            for name, cmd in peers:
                a = ask(cmd, {"op": op, "source": case["source"], "path": case.get("path")})
                out["answers"][f"{case['id']}/{op}/{name}"] = render(a)
    return out


def compare(record: dict, fresh: dict) -> int:
    """Drift is a *same version* giving a *different answer*.

    A different version is not drift, it is a different question, and
    failing on it would make the check mean "does your machine match the
    one that wrote the record" rather than "has anything changed". Both
    are reported; only the first is an error.
    """
    drift, skipped = [], []
    for name, fresh_v in fresh["versions"].items():
        rec_v = record["versions"].get(name)
        if rec_v is None:
            skipped.append(f"{name}: not in the record ({fresh_v})")
            continue
        if rec_v != fresh_v:
            # Not drift, but the answers are still worth showing: an upgrade
            # changing an answer a spec leans on is the whole point of
            # keeping the record, and it must not be silent just because it
            # cannot be an error.
            moved = [
                k
                for k, a in fresh["answers"].items()
                if k.endswith(f"/{name}") and record["answers"].get(k) != a
            ]
            note = f"{name}: record has {rec_v}, this machine has {fresh_v}"
            if moved:
                note += f"; {len(moved)} answer(s) differ, starting with {moved[0]}"
            skipped.append(note)
            continue
        for key, fresh_a in fresh["answers"].items():
            if not key.endswith(f"/{name}"):
                continue
            rec_a = record["answers"].get(key)
            if rec_a != fresh_a:
                drift.append(f"{key}\n    record: {rec_a}\n    now:    {fresh_a}")
    for line in skipped:
        print(f"not comparable, {line}", file=sys.stderr)
    for line in drift:
        print(f"DRIFT {line}", file=sys.stderr)
    if drift:
        print(
            f"\n{len(drift)} answer(s) changed on an unchanged library version.",
            file=sys.stderr,
        )
        print("Re-run without --check, read the diff, and record why.", file=sys.stderr)
        return 1
    comparable = len(fresh["versions"]) - len(skipped)
    print(f"testbed: {comparable} of {len(fresh['versions'])} implementations compared, no drift")
    return 0


def main() -> int:
    check = "--check" in sys.argv
    import yaml  # PyYAML, which the testbed requires anyway

    cases = yaml.safe_load((HERE / "cases.yaml").read_text())
    with tempfile.TemporaryDirectory() as td:
        peers = [(n, build_go(Path(td)) if c is None else c) for n, c in PEERS]
        peers = [(n, c) for n, c in peers if c is not None] or []
        fresh = collect(peers, cases)
        versions = [f"| {n} | {v} |" for n, v in fresh["versions"].items()]

        out = [
            "# Reference testbed: recorded answers",
            "",
            "Generated by `python3 testbed/run.py`. Do not edit by hand.",
            "",
            "Each row is one implementation's answer to one question about one",
            "document. A refusal is an answer, and so is `unsupported`: it is why a",
            "comment question cannot be settled by citing PyYAML.",
            "",
            "| implementation | version |",
            "|---|---|",
            *versions,
            "",
        ]
        for case in cases:
            out += [f"## {case['id']}", "", case["why"].strip(), "", "```yaml",
                    case["source"].replace("\r\n", "\\r\\n ->\n"), "```", ""]
            if "path" in case:
                out += [f"Path: `{json.dumps(case['path'])}`", ""]
            for op in case["ops"]:
                out += [f"### {op}", "", "| implementation | answer |", "|---|---|"]
                for name, _ in peers:
                    out.append(f"| {name} | {fresh['answers'][f"{case['id']}/{op}/{name}"]} |")
                out.append("")

        text = "\n".join(out) + "\n"

    record_path = HERE / "answers.json"
    report_path = HERE / "answers.md"
    if check:
        if not record_path.exists():
            print("no record yet; run without --check first", file=sys.stderr)
            return 1
        return compare(json.loads(record_path.read_text()), fresh)
    record_path.write_text(json.dumps(fresh, indent=2, sort_keys=True) + "\n")
    report_path.write_text(text)
    print(f"testbed: wrote {record_path.name} and {report_path.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
