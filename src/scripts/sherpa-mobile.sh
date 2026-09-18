#!/usr/bin/env bash
# Fetches the sherpa-onnx runtime libraries for a phone: the shared
# libraries k2-fsa publishes, which the app carries with it.
#
#   scripts/sherpa-mobile.sh aarch64-linux-android [out-dir]
#
# The version is the one the lockfile resolved for sherpa-onnx-sys, so a
# bump of the crate cannot desynchronise from the libraries.
#
# Android: vendor/sherpa-onnx/jniLibs/<abi>/*.so, the layout an APK's
# native library directory takes, with the NDK's libc++_shared.so beside
# them because onnxruntime loads it. Point SHERPA_ONNX_LIB_DIR at the ABI
# directory to build, and cargo-apk packages the whole tree through
# concat-android's manifest.
set -euo pipefail

target=${1:?target triple: aarch64-linux-android}
workspace=$(cd "$(dirname "$0")/.." && pwd)
version=$(grep -A1 'name = "sherpa-onnx-sys"' "$workspace/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p')
release=https://github.com/k2-fsa/sherpa-onnx/releases/download

case "$target" in
  aarch64-linux-android)
    out=${2:-$workspace/vendor/sherpa-onnx}
    ndk=${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}
    if [ -z "$ndk" ]; then
      sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}
      ndk=$(ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -1 || true)
    fi
    [ -d "$ndk" ] || { echo "no Android NDK: set ANDROID_NDK_HOME" >&2; exit 1; }
    host_tag=$(ls "$ndk/toolchains/llvm/prebuilt" | head -1)
    echo "Fetching sherpa-onnx $version for Android"
    mkdir -p "$out"
    curl -fsSL --retry 5 --retry-all-errors -o "$out/sherpa-onnx-android.tar.bz2" \
      "$release/v$version/sherpa-onnx-v$version-android.tar.bz2"
    tar -xjf "$out/sherpa-onnx-android.tar.bz2" -C "$out"
    rm "$out/sherpa-onnx-android.tar.bz2"
    # Only the ABI the app is built for ships; the rest is left behind.
    find "$out/jniLibs" -mindepth 1 -maxdepth 1 -type d ! -name arm64-v8a -exec rm -rf {} +
    # The C API and its runtime are what the engine loads; the JNI and
    # C++ bindings are for other languages.
    rm -f "$out"/jniLibs/arm64-v8a/libsherpa-onnx-jni.so "$out"/jniLibs/arm64-v8a/libsherpa-onnx-cxx-api.so
    cp "$ndk/toolchains/llvm/prebuilt/$host_tag/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so" \
      "$out/jniLibs/arm64-v8a/"
    echo "sherpa-onnx for Android is in $out/jniLibs"
    echo "  export SHERPA_ONNX_LIB_DIR=$out/jniLibs/arm64-v8a"
    ;;
  *)
    echo "unknown target $target" >&2; exit 1 ;;
esac
