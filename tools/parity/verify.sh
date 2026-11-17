#!/usr/bin/env bash
set -euo pipefail

parity_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixtures="$parity_dir/fixtures"
manifest="$fixtures/MANIFEST.sha256"

if [[ ! -f "$manifest" ]]; then
  printf 'verify: missing %s\n' "$manifest" >&2
  exit 1
fi

(cd "$parity_dir" && sha256sum -c "$manifest")

state_count="$(find "$fixtures" -maxdepth 1 -type f -name 'state-*.json' | wc -l)"
evidence_count="$(find "$fixtures" -maxdepth 1 -type d -name 'evidence-*' | wc -l)"

if [[ "$state_count" -ne 200 ]]; then
  printf 'verify: expected 200 state fixtures, found %s\n' "$state_count" >&2
  exit 1
fi
if [[ "$evidence_count" -ne 20 ]]; then
  printf 'verify: expected 20 evidence directories, found %s\n' "$evidence_count" >&2
  exit 1
fi

printf 'verify: checksums OK, 200 state fixtures, 20 evidence directories\n'
