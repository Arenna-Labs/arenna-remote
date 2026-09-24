#!/usr/bin/env bash
# Configure release signing for the Flutter Android build.
#
# With the ANDROID_* secrets present, writes flutter/android/key.properties so
# Gradle signs the release APK with the Arenna Remote keystore (every release
# must use the same key, or Android refuses to install it over the previous
# one). Without them, test builds fall back to the debug key; publishing
# without them is an error.
set -euo pipefail

PUBLISH="${1:-false}"

if [ -z "${ANDROID_SIGNING_KEY:-}" ]; then
  if [ "$PUBLISH" = "true" ]; then
    echo "Android signing secrets are required to publish a release" >&2
    exit 1
  fi
  echo "::warning::ANDROID_SIGNING_KEY not set, the APK will be debug-signed"
  sed -i "s/signingConfigs.release/signingConfigs.debug/g" flutter/android/app/build.gradle
  exit 0
fi

KEYSTORE="${RUNNER_TEMP:-/tmp}/arenna-remote-release.jks"
echo "$ANDROID_SIGNING_KEY" | base64 -d > "$KEYSTORE"
cat > flutter/android/key.properties <<PROPS
storePassword=${ANDROID_KEY_STORE_PASSWORD}
keyPassword=${ANDROID_KEY_PASSWORD}
keyAlias=${ANDROID_ALIAS}
storeFile=${KEYSTORE}
PROPS
echo "Release signing configured with alias ${ANDROID_ALIAS}"
