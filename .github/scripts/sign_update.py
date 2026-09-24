#!/usr/bin/env python3
"""Sign or verify Arenna Remote update files with Ed25519.

The Windows client downloads `<asset>` and `<asset>.sig` from the GitHub
release and refuses to run the installer unless one of the signatures
verifies against one of the public keys compiled into the app
(`hbb_common::arenna::UPDATE_PUBLIC_KEYS`).

Signed message: the asset file name, a newline, then the file bytes. Binding
the name (which carries the version) stops an older signed installer from
being served as a newer one.

Usage:
    sign_update.py sign FILE...              # keys: env UPDATE_SIGNING_KEY
    sign_update.py verify FILE PUBKEY...     # PUBKEY: base64 32-byte public key
    sign_update.py keygen                    # prints a new seed and its public key

UPDATE_SIGNING_KEY holds one or more base64 32-byte seeds separated by
whitespace or commas; `sign` writes FILE.sig with one base64 signature per
key and line (several keys are used while rotating the signing key).
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


def load_private_keys():
    seeds = os.environ.get("UPDATE_SIGNING_KEY", "").replace(",", " ").split()
    if not seeds:
        sys.exit("UPDATE_SIGNING_KEY is not set")
    keys = []
    for seed_b64 in seeds:
        seed = base64.b64decode(seed_b64)
        if len(seed) != 32:
            sys.exit("UPDATE_SIGNING_KEY must hold base64 32-byte Ed25519 seeds")
        keys.append(Ed25519PrivateKey.from_private_bytes(seed))
    return keys


def signed_message(path) -> bytes:
    with open(path, "rb") as f:
        return os.path.basename(path).encode("utf-8") + b"\n" + f.read()


def sign(paths):
    keys = load_private_keys()
    for path in paths:
        message = signed_message(path)
        lines = [base64.b64encode(key.sign(message)).decode("ascii") for key in keys]
        with open(path + ".sig", "w", encoding="ascii") as f:
            f.write("\n".join(lines) + "\n")
        print(f"signed {path} with {len(keys)} key(s)")


def verify(path, pubkeys_b64):
    public_keys = [Ed25519PublicKey.from_public_bytes(base64.b64decode(k)) for k in pubkeys_b64]
    with open(path + ".sig", "r", encoding="ascii") as f:
        signatures = [base64.b64decode(line) for line in f.read().split()]
    message = signed_message(path)
    for public_key in public_keys:
        for signature in signatures:
            try:
                public_key.verify(signature, message)
                print(f"valid signature for {path}")
                return
            except InvalidSignature:
                pass
    sys.exit(f"INVALID signature for {path}")


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
    elif len(argv) >= 3 and argv[0] == "verify":
        verify(argv[1], argv[2:])
    elif argv == ["keygen"]:
        keygen()
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
