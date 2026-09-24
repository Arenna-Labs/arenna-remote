#!/usr/bin/env python3
"""Print the update public keys compiled into the app (UPDATE_PUBLIC_KEYS in
libs/hbb_common/src/arenna.rs), space separated, so CI can check that a
release signature will be accepted by the clients."""

import re
import sys

source = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r"UPDATE_PUBLIC_KEYS:\s*&\[&str\]\s*=\s*&\[(.*?)\];", source, re.S)
if not match:
    sys.exit("UPDATE_PUBLIC_KEYS not found")
keys = re.findall(r'"([A-Za-z0-9+/=]{44})"', match.group(1))
if not keys:
    sys.exit("no keys in UPDATE_PUBLIC_KEYS")
print(" ".join(keys))
