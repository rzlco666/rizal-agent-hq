#!/usr/bin/env bash
set -euo pipefail

# Freeze the live v2 oracle: 200 /api/state samples plus raw evidence.
# Run from anywhere; artifacts are written next to this script.
parity_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixtures="$parity_dir/fixtures"
oracle="$parity_dir/oracle"
endpoint="http://127.0.0.1:8900/api/state"
repo_dir="$(cd -- "$parity_dir/../.." && pwd)"
duration=1200
interval=6

usage() {
  printf 'usage: %s [--duration SECONDS] [--interval SECONDS]\n' "$0"
}

while (($#)); do
  case "$1" in
    --duration)
      shift
      duration="${1:?--duration needs a value}"
      shift
      ;;
    --interval)
      shift
      interval="${1:?--interval needs a value}"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! [[ "$duration" =~ ^[1-9][0-9]*$ && "$interval" =~ ^[1-9][0-9]*$ ]]; then
  printf 'duration and interval must be positive integers\n' >&2
  exit 2
fi

samples=$(( (duration + interval - 1) / interval ))
mkdir -p "$fixtures"

write_evidence() {
  local stamp="$1"
  local dir="$fixtures/evidence-$stamp"
  mkdir -p "$dir"
  # Komando bukti dibatasi waktu: omp stats dapat menggantung bila runtime sibuk.
  # Argumen ps sengaja dihilangkan karena dapat memuat secret.
  timeout 10s ps -eo pid,ppid,etime,stat,comm > "$dir/ps.txt" 2>/dev/null || true
  timeout 5s omp stats > "$dir/omp-stats.txt" 2>&1 || true
  timeout 10s tmux ls > "$dir/tmux.txt" 2>&1 || true
  timeout 10s docker ps > "$dir/docker.txt" 2>&1 || true
  timeout 10s git -C "$repo_dir" log -1 > "$dir/git.txt" 2>&1 || true
  timeout 10s find "$HOME/.omp" "$HOME/.cursor" "$HOME/.local/share/opencode" \
    -maxdepth 2 -type f -name '*.json*' -print 2>/dev/null \
    | head -200 > "$dir/sessions.txt" || true
}

for ((i = 1; i <= samples; i++)); do
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  tmp="$(mktemp "$fixtures/.state-$stamp.XXXXXX")"
  if ! curl -fsS "$endpoint" > "$tmp"; then
    rm -f "$tmp"
    printf 'freeze: GET %s failed at sample %d/%d\n' "$endpoint" "$i" "$samples" >&2
    exit 1
  fi
  mv "$tmp" "$fixtures/state-$stamp.json"
  if ((i <= 20)); then
    write_evidence "$stamp"
  fi
  if ((i < samples)); then
    sleep "$interval"
  fi
done

manifest_tmp="$(mktemp "$fixtures/.MANIFEST.XXXXXX")"
(
  cd "$parity_dir"
  find oracle fixtures -type f ! -name 'MANIFEST.sha256' ! -name '.*' -print0 \
    | sort -z \
    | xargs -0 sha256sum > "$manifest_tmp"
)
mv "$manifest_tmp" "$fixtures/MANIFEST.sha256"
printf 'freeze: %d state fixtures, 20 evidence directories, manifest written\n' "$samples"
