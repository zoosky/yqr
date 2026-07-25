---
menu:
  order: 2
---
# yqr demo

A runnable showcase of yqr, a jq-style query & transform tool for YAML.

## Run it

```bash
bash yqr-demo.sh
```

The script needs `yqr` on your `PATH`. It resolves its own directory, so it
works from any working directory and reads the sample files in place.

## What's in here

| File            | Role                                                              |
|-----------------|-------------------------------------------------------------------|
| `yqr-demo.sh`   | The narrated walkthrough (seven sections, each a real query).     |
| `deploy.yaml`   | A Kubernetes Deployment -- the input for navigation & iteration.  |
| `config.yaml`   | A hand-commented config -- the input for the fidelity engine.     |

## What it shows

1. Navigate nested structure -- dotted paths and array indexing (`.spec.containers[0].image`, negative indices).
2. Iterate collections -- `[]` streams every element.
3. Compose with pipes -- `|` feeds one filter into the next.
4. Raw output -- `-r` drops YAML quoting for shell scripting.
5. Reads from stdin -- pipe YAML straight in.
6. Fidelity by default -- `yqr '.'` reproduces the input byte-for-byte, comments and all, with no flag. `--normalize` opts into the classic re-serializing pipeline; `--engine` picks the backend parser for the default read (default `noyalib`).
7. jq-style exit codes -- `3` for parse errors, `5` for runtime errors, for scriptable error handling.
