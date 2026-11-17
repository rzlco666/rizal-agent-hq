#!/usr/bin/env bash
set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
parity_dir="$(cd -- "$here/.." && pwd)"

python3 - "$parity_dir/aliases.json" <<'PY'
import json
import sys

expected = {
    "omp": "agent:omp",
    "cursor": "agent:cursor",
    "Muse": "agent:muse",
    "mcp": "agent:mcp/*",
    "omp-workers-agg": "agent:omp/worker/*",
    "coding-agents": "agent:*",
}

with open(sys.argv[1], encoding="utf-8") as handle:
    actual = json.load(handle)

missing = {key: value for key, value in expected.items() if actual.get(key) != value}
if missing:
    for key, value in sorted(missing.items()):
        print(f"FAIL: aliases.json maps {key!r} to {actual.get(key)!r}, expected {value!r}", file=sys.stderr)
    sys.exit(1)

print("PASS: aliases.json contains all six canonical v2 -> v3 mappings")
PY
