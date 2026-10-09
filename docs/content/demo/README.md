---
lead: >-
  A runnable script that walks yqr's query, edit, and validate surface against sample files you can inspect afterwards.
menu:
  order: 4
---
# yqr demo

A runnable showcase of yqr, a YAML editor with a fidelity guarantee and
jq-style ergonomics.

## Run it

```bash
bash yqr-demo.sh
```

The script needs `yqr` on your `PATH` (`brew install zoosky/tap/yqr`, or
`cargo install yqr`). It resolves its own directory, so it works from any
working directory and reads the sample files in place; every mutating
command works on a copy in a scratch directory, so re-running it is
idempotent.

## What's in here

| File                                                | Role                                                              |
|-----------------------------------------------------|-------------------------------------------------------------------|
| [`yqr-demo.sh`](https://github.com/zoosky/yqr/blob/main/docs/content/demo/yqr-demo.sh) | The narrated walkthrough (eleven sections, each a real command). Linked on GitHub -- accent does not serve script files as page media. |
| [`deploy.yaml`](/content-media/demo/deploy.yaml)    | A Kubernetes Deployment -- the input for navigation & iteration.  |
| [`config.yaml`](/content-media/demo/config.yaml)    | A hand-commented config -- the input for fidelity, edits, and validate. |
| [`services.yaml`](/content-media/demo/services.yaml) | Anchors, a merge key, and alias entries -- the input for the alias edits. |

## What it shows

1. Navigate nested structure -- dotted paths and array indexing (`.spec.containers[0].image`, negative indices).
2. Iterate collections -- `[]` streams every element.
3. Compose with pipes -- `|` feeds one filter into the next.
4. Raw output -- `-r` drops YAML quoting for shell scripting.
5. Reads from stdin -- pipe YAML straight in.
6. Fidelity by default -- `yqr '.'` reproduces the input byte-for-byte, comments and all, with no flag. `--normalize` opts into the classic re-serializing pipeline.
7. Edit without reformatting -- a write replaces one value and the diff is that line; append to a sequence, delete an entry structurally, and every surviving byte is identical.
8. Anchors, aliases, merge keys -- a key inherited through `<<:` takes an explicit override, an entry whose value is an alias is replaced or deleted through its own `*name` token, and writing the value an alias already resolves to keeps your `*name` spelling.
9. Comments are addressable -- read the comment on a value with `line_comment(...)`, or write one.
10. jq-style exit codes -- `3` for parse errors, `5` for runtime errors, for scriptable error handling.
11. Validate after editing -- `yqr validate` confirms an edited file is still correct YAML (silent, exit 0); break it and the verdict is a compiler-style diagnostic with a stable code, the offending line, and a caret. `--strict` additionally catches a duplicate key, and `--schema` holds the values to a JSON Schema with the finding located in your file.
