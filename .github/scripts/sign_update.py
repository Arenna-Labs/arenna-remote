#!/usr/bin/env python3
"""Sign or verify Arenna Remote update files with Ed25519.

The Windows client downloads `<asset>` and `<asset>.sig` from the GitHub
release and refuses to run the installer unless the signature verifies
against the public key compiled into the app
(`hbb_common::arenna::UPDATE_PUBLIC_KEY`).

Usage:
    sign_update.py sign FILE...          # key: env UPDATE_SIGNING_KEY (base64 32-byte seed)
    sign_update.py verify FILE PUBKEY    # PUBKEY: base64 32-byte public key
    sign_update.py keygen                # prints a new seed and its public key

`sign` writes FILE.sig containing the base64 signature and a newline.
"""

import base64
import os
import sys

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)


def load_private_key() -> Ed25519PrivateKey:
    seed_b64 = os.environ.get("UPDATE_SIGNING_KEY", "").strip()
    if not seed_b64:
        sys.exit("UPDATE_SIGNING_KEY is not set")
    seed = base64.b64decode(seed_b64)
    if len(seed) != 32:
        sys.exit("UPDATE_SIGNING_KEY must be a base64 32-byte Ed25519 seed")
    return Ed25519PrivateKey.from_private_bytes(seed)


def sign(paths):
    key = load_private_key()
    for path in paths:
        with open(path, "rb") as f:
            signature = key.sign(f.read())
        with open(path + ".sig", "w", encoding="ascii") as f:
            f.write(base64.b64encode(signature).decode("ascii") + "\n")
        print(f"signed {path}")


def verify(path, pubkey_b64):
    public_key = Ed25519PublicKey.from_public_bytes(base64.b64decode(pubkey_b64))
    with open(path + ".sig", "r", encoding="ascii") as f:
        signature = base64.b64decode(f.read().strip())
    with open(path, "rb") as f:
        data = f.read()
    try:
        public_key.verify(signature, data)
    except InvalidSignature:
        sys.exit(f"INVALID signature for {path}")
    print(f"valid signature for {path}")


def keygen():
    key = Ed25519PrivateKey.generate()
    seed = key.private_bytes(
        serialization.Encoding.Raw,
        serialization.PrivateFormat.Raw,
        serialization.NoEncryption(),
    )
    public = key.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    print("UPDATE_SIGNING_KEY=" + base64.b64encode(seed).decode("ascii"))
    print("UPDATE_PUBLIC_KEY=" + base64.b64encode(public).decode("ascii"))


def main(argv):
    if len(argv) >= 2 and argv[0] == "sign":
        sign(argv[1:])
    elif len(argv) == 3 and argv[0] == "verify":
        verify(argv[1], argv[2])
    elif argv == ["keygen"]:
        keygen()
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
