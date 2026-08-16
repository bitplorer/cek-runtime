#!/usr/bin/env sh
# Publish crates.io in dependency order.
#   DRY_RUN=1 sh scripts/publish_crates.sh     # default
#   DRY_RUN=0 sh scripts/publish_crates.sh     # needs CARGO_REGISTRY_TOKEN
set -eu
cd "$(dirname "$0")/.."
DRY_RUN="${DRY_RUN:-1}"
flag="--dry-run"
if [ "$DRY_RUN" = "0" ]; then
  flag=""
  if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
    echo "CARGO_REGISTRY_TOKEN is required for a live publish" >&2
    exit 2
  fi
fi

# leaf → dependents
for crate in \
  cek-contract \
  cek-ops-baseline \
  cek-ops-ui \
  cek-host-kernel \
  cek-peer-kernel \
  cek-peer-wasm \
  cek-cli
do
  echo "=== $crate $flag ==="
  cargo publish -p "$crate" --locked $flag
done
