#!/usr/bin/env bash
# L7 demo: Python Host → contract JSON → JS Peer.
set -euo pipefail
cd "$(dirname "$0")"
python3 orchestrate.py
