#!/usr/bin/env bash
# Checks that the APK carries the native libraries the app loads at startup.
#
# The build can succeed without them: rules_android drops C dependencies it
# decides are empty (see //bindings/uniffi:calc_ffi_android_native), and the
# only symptom is a crash on launch. The archive is listed with Bazel's own
# zip tool, so no unzip is needed on the host.
#
# Usage: apk_contents_test.sh <zipper> <apk> <path inside the apk>...
set -euo pipefail

zipper="$1"
apk="$2"
shift 2

listing="$("$zipper" v "$apk")"
failed=0
for path in "$@"; do
  # zipper's listing is one entry per line: "f <mode> <path>".
  if ! grep -q " $path\$" <<<"$listing"; then
    echo "missing from $apk: $path" >&2
    failed=1
  fi
done
if [ "$failed" -ne 0 ]; then
  echo "APK contents:" >&2
  echo "$listing" >&2
fi
exit "$failed"
