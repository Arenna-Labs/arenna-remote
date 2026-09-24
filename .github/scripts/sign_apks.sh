#!/usr/bin/env bash
# Re-sign debug-signed APKs with the Arenna Remote release keystore.
#
#   sign_apks.sh APK...
#
# Needs ANDROID_SIGNING_KEY (base64 keystore), ANDROID_ALIAS,
# ANDROID_KEY_STORE_PASSWORD, ANDROID_KEY_PASSWORD and the Android SDK
# build-tools. Every release must use the same key, or Android refuses to
# install it over the previous version.
set -euo pipefail

: "${ANDROID_SIGNING_KEY:?missing}" "${ANDROID_ALIAS:?missing}"
: "${ANDROID_KEY_STORE_PASSWORD:?missing}" "${ANDROID_KEY_PASSWORD:?missing}"

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/usr/local/lib/android/sdk}}"
BUILD_TOOLS="$SDK/build-tools/$(ls "$SDK/build-tools" | sort -V | tail -n 1)"
KEYSTORE="$(mktemp -d)/release.jks"
trap 'rm -f "$KEYSTORE"' EXIT
echo "$ANDROID_SIGNING_KEY" | base64 -d > "$KEYSTORE"

for apk in "$@"; do
  aligned="${apk%.apk}.aligned.apk"
  "$BUILD_TOOLS/zipalign" -p -f 4 "$apk" "$aligned"
  "$BUILD_TOOLS/apksigner" sign --ks "$KEYSTORE" --ks-key-alias "$ANDROID_ALIAS" \
    --ks-pass env:ANDROID_KEY_STORE_PASSWORD --key-pass env:ANDROID_KEY_PASSWORD \
    --out "$apk" "$aligned"
  rm -f "$aligned" "$apk.idsig"
  "$BUILD_TOOLS/apksigner" verify --print-certs "$apk" | grep -E "Signer #1 certificate DN|SHA-256"
done
