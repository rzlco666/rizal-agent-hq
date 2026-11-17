#!/usr/bin/env bash
set -euo pipefail

# Usage: ci-config.sh [repo-root] — defaults to the checkout containing this script.
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root="${1:-$(cd -- "$script_dir/../.." && pwd)}"
workflow="$root/.github/workflows/ci.yml"

required=(
  .github/workflows/ci.yml
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  deny.toml
  tools/checks/ci-config.sh
  tools/parity/freeze.sh
  tools/parity/verify.sh
  tools/parity/aliases.json
  tools/parity/normalize.md
  tools/parity/fixtures/MANIFEST.sha256
)

for path in "${required[@]}"; do
  if [[ ! -f "$root/$path" ]]; then
    printf 'ci-config: missing %s\n' "$path" >&2
    exit 1
  fi
done

python3 - "$workflow" <<'PY'
import sys

import yaml

with open(sys.argv[1], encoding="utf-8") as handle:
    config = yaml.safe_load(handle)

if not isinstance(config, dict):
    sys.exit("ci-config: workflow must be a YAML mapping")

jobs = config.get("jobs")
if not isinstance(jobs, dict) or not {"rust", "parity"} <= set(jobs):
    sys.exit("ci-config: workflow must define rust and parity jobs")

problems = []


def walk(node, path):
    if isinstance(node, dict):
        for key, value in node.items():
            if str(key) == "continue-on-error" and (
                value is True or (isinstance(value, str) and value.lower() == "true")
            ):
                problems.append(f"{path}.{key}")
            walk(value, f"{path}.{key}")
    elif isinstance(node, list):
        for index, value in enumerate(node):
            walk(value, f"{path}[{index}]")


walk(config, "$")

if problems:
    for path in problems:
        print(f"ci-config: {path} must not be true", file=sys.stderr)
    sys.exit(1)
PY

printf 'ci-config: workflow valid, required files present, no continue-on-error\n'
