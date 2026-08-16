#!/usr/bin/env sh
# Publish crates.io in dependency order.
#   DRY_RUN=1 sh scripts/publish_crates.sh     # default
#   DRY_RUN=0 sh scripts/publish_crates.sh     # needs CARGO_REGISTRY_TOKEN
set -eu
cd "$(dirname "$0")/.."
DRY_RUN="${DRY_RUN:-1}"

if [ "$DRY_RUN" = "0" ] && [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "CARGO_REGISTRY_TOKEN is required for a live publish" >&2
  exit 2
fi

publish_one() {
  crate=$1
  if [ "$DRY_RUN" != "0" ]; then
    echo "=== $crate --dry-run ==="
    cargo publish -p "$crate" --dry-run --allow-dirty
    return 0
  fi
  echo "=== $crate ==="
  i=1
  while [ "$i" -le 8 ]; do
    if out=$(cargo publish -p "$crate" --allow-dirty 2>&1); then
      echo "$out"
      return 0
    fi
    echo "$out"
    if echo "$out" | grep -qiE 'already (uploaded|exists|published)'; then
      echo "skip $crate (already on crates.io)"
      return 0
    fi
    if echo "$out" | grep -qiE 'too many new crates|Too Many Requests|429'; then
      echo "crates.io rate limit; wait 70s ($i/8)"
      sleep 70
      i=$((i + 1))
      continue
    fi
    if echo "$out" | grep -qiE 'no matching package named|failed to select a version'; then
      echo "index lag; retry $i/8 in 20s"
      sleep 20
      i=$((i + 1))
      continue
    fi
    return 1
  done
  echo "gave up publishing $crate" >&2
  return 1
}

for crate in \
  cek-contract \
  cek-ops-baseline \
  cek-ops-ui \
  cek-host-kernel \
  cek-peer-kernel \
  cek-peer-wasm \
  cek-cli
do
  publish_one "$crate"
done
