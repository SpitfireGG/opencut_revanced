#!/usr/bin/env bash
# Puts the Android build of HEAD on the phone over adb - no tag, no
# release, nothing to download on the phone. Push main, then:
#
#   scripts/phone.sh          # wait for HEAD's build, install it, open it
#   scripts/phone.sh --logs   # the same, then follow the app's log
#
# The phone must show in `adb devices`: a USB cable with USB debugging on,
# or over Wi-Fi with Settings > Developer options > Wireless debugging and
# `adb pair <ip:port>` once, then `adb connect <ip:port>`.
set -euo pipefail

sha=$(git rev-parse HEAD)
run=$(gh run list --workflow android.yml --commit "$sha" -L 1 --json databaseId -q '.[0].databaseId')
[ -n "$run" ] || { echo "no Android build for ${sha:0:7}: push main first" >&2; exit 1; }
adb get-state >/dev/null || { echo "no phone in adb devices" >&2; exit 1; }

echo "waiting for build $run (${sha:0:7})"
gh run watch "$run" --exit-status --interval 20 >/dev/null

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
gh run download "$run" -n Concat-android-arm64 -D "$dir"
# -r keeps the app's projects: every build is signed with the repository's
# key, so each one updates the last.
adb install -r "$dir"/*.apk
adb shell monkey -p app.concat.editor 1 >/dev/null

if [ "${1:-}" = --logs ]; then
  adb logcat -c
  exec adb logcat -s concat AndroidRuntime
fi
