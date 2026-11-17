#!/usr/bin/env bash
set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
parity_dir="$(cd -- "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cp "$parity_dir/verify.sh" "$tmp/verify.sh"
chmod +x "$tmp/verify.sh"
mkdir -p "$tmp/fixtures"

for n in $(seq 1 200); do
  printf '{"sample":%s}\n' "$n" > "$tmp/fixtures/state-synthetic-$n.json"
done
for n in $(seq 1 20); do
  mkdir -p "$tmp/fixtures/evidence-synthetic-$n"
  printf 'sample %s\n' "$n" > "$tmp/fixtures/evidence-synthetic-$n/ps.txt"
done

make_manifest() {
  (cd "$tmp" && find fixtures -type f ! -name 'MANIFEST.sha256' -print0 | sort -z | xargs -0 sha256sum > fixtures/MANIFEST.sha256)
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

make_manifest
(cd "$tmp" && ./verify.sh) > /dev/null || fail 'verify.sh rejected an intact synthetic fixture set'

printf 'tampered\n' >> "$tmp/fixtures/state-synthetic-200.json"
if (cd "$tmp" && ./verify.sh) > /dev/null 2>&1; then
  fail 'verify.sh accepted a tampered state fixture'
fi
printf 'PASS: verify.sh rejects a tampered state fixture\n'

make_manifest
rm "$tmp/fixtures/state-synthetic-200.json"
if (cd "$tmp" && ./verify.sh) > /dev/null 2>&1; then
  fail 'verify.sh accepted 199 state fixtures'
fi
printf 'PASS: verify.sh rejects 199 state fixtures\n'

for n in $(seq 1 200); do
  printf '{"sample":%s}\n' "$n" > "$tmp/fixtures/state-synthetic-$n.json"
done
make_manifest
rm -rf "$tmp/fixtures/evidence-synthetic-20"
if (cd "$tmp" && ./verify.sh) > /dev/null 2>&1; then
  fail 'verify.sh accepted 19 evidence directories'
fi
printf 'PASS: verify.sh rejects 19 evidence directories\n'
