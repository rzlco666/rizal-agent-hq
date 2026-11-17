#!/usr/bin/env bash
set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
checker="$(cd -- "$here/.." && pwd)/ci-config.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/.github/workflows" "$tmp/tools/checks" "$tmp/tools/parity"
for path in Cargo.toml Cargo.lock rust-toolchain.toml deny.toml \
  tools/checks/ci-config.sh tools/parity/freeze.sh tools/parity/verify.sh \
  tools/parity/aliases.json tools/parity/normalize.md; do
  printf 'placeholder\n' > "$tmp/$path"
done

valid_yaml() {
  cat <<'YAML'
name: CI
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --workspace
  parity:
    runs-on: ubuntu-latest
    steps:
      - run: bash tools/parity/verify.sh
YAML
}

valid_yaml > "$tmp/.github/workflows/ci.yml"
if ! bash "$checker" "$tmp" > /dev/null; then
  printf 'FAIL: valid CI configuration was rejected\n' >&2
  exit 1
fi
printf 'PASS: valid CI configuration accepted\n'

{
  valid_yaml
  printf '  continue-on-error: true\n'
} > "$tmp/.github/workflows/ci.yml"
if bash "$checker" "$tmp" > /dev/null 2>&1; then
  printf 'FAIL: continue-on-error: true was accepted\n' >&2
  exit 1
fi
printf 'PASS: continue-on-error: true rejected\n'

printf 'jobs: [\n' > "$tmp/.github/workflows/ci.yml"
if bash "$checker" "$tmp" > /dev/null 2>&1; then
  printf 'FAIL: malformed YAML was accepted\n' >&2
  exit 1
fi
printf 'PASS: malformed YAML rejected\n'
