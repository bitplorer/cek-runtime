#!/usr/bin/env python3
"""Host runtime: stdin Intent JSON → stdout Result. Decide only — no apply."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from cek_host import Host, attach_ed25519, attach_hmac

if "--help" in sys.argv or "-h" in sys.argv:
    print("usage: python3 runtime.py [--now N] [--hmac-hex KEY] [--ed25519-hex SEED] < intent.json")
    raise SystemExit(0)

now = 0
hmac_key = None
ed_seed = None
args = sys.argv[1:]
i = 0
while i < len(args):
    if args[i] == "--now":
        now = int(args[i + 1])
        i += 2
    elif args[i] == "--hmac-hex":
        hmac_key = bytes.fromhex(args[i + 1])
        i += 2
    elif args[i] == "--ed25519-hex":
        ed_seed = bytes.fromhex(args[i + 1])
        i += 2
    else:
        print("unknown flag", args[i], file=sys.stderr)
        raise SystemExit(2)

intent = json.load(sys.stdin)
host = Host(now=now, hmac_key=hmac_key, ed25519_seed=ed_seed)
print(json.dumps(host.submit(intent), indent=2))
