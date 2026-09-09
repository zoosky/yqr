// js-yaml adapter, run on Bun.
//
// js-yaml is what most of the JavaScript world parses YAML with, so it is
// a fifth independent judge of whether a document is well formed. Like
// PyYAML and Psych it keeps no comments, so it answers `load` and
// `roundtrip` only.
import yaml from "js-yaml";

type Req = { op: string; source?: string; path?: (string | number)[] };

function canon(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(canon);
  if (v && typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, x] of Object.entries(v as object)) out[String(k)] = canon(x);
    return out;
  }
  return v;
}

const req: Req = JSON.parse(await Bun.stdin.text());
let out: Record<string, unknown>;
if (req.op === "version") {
  const v = (await import("js-yaml/package.json")).default.version;
  out = { version: `js-yaml ${v}` };
} else {
  try {
    switch (req.op) {
      case "load":
        out = { result: JSON.stringify(canon(yaml.load(req.source!))) };
        break;
      case "roundtrip":
        out = { result: yaml.dump(yaml.load(req.source!)) };
        break;
      case "comments":
      case "delete":
        out = { unsupported: "js-yaml keeps no comments and has no editing model" };
        break;
      default:
        out = { error: `unknown op ${req.op}` };
    }
  } catch (e) {
    const m = e instanceof Error ? e.message.split("\n")[0] : String(e);
    out = { error: `${e instanceof Error ? e.name : "Error"}: ${m}` };
  }
}
console.log(JSON.stringify(out));
