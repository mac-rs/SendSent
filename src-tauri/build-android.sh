#!/usr/bin/env bash
# Build the Rust core for Android and drop the .so into the Gradle module.
# Usage: build-android.sh [<rust-target> <abi>]   (default: aarch64 / arm64-v8a)
set -euo pipefail

TARGET="${1:-aarch64-linux-android}"
ABI="${2:-arm64-v8a}"

: "${ANDROID_HOME:?ANDROID_HOME must be set}"
if [[ -z "${NDK_HOME:-}" ]]; then
  NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1)"
fi
: "${NDK_HOME:?NDK_HOME not found under \$ANDROID_HOME/ndk}"

case "$(uname -s)" in
  Darwin) HOST_TAG=darwin-x86_64 ;;
  Linux)  HOST_TAG=linux-x86_64 ;;
  *) echo "unsupported host" >&2; exit 1 ;;
esac
TC="$NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG"
export PATH="$TC/bin:$PATH"

API=24
case "$TARGET" in
  aarch64-linux-android)          CLANG=aarch64-linux-android${API}-clang ;;
  armv7-linux-androideabi)        CLANG=armv7a-linux-androideabi${API}-clang ;;
  i686-linux-android)             CLANG=i686-linux-android${API}-clang ;;
  x86_64-linux-android)           CLANG=x86_64-linux-android${API}-clang ;;
  *) echo "unknown target $TARGET" >&2; exit 1 ;;
esac

ENV_SUFFIX="$(echo "$TARGET" | tr 'a-z-' 'A-Z_')"
export "CARGO_TARGET_${ENV_SUFFIX}_LINKER=$TC/bin/$CLANG"
export "CC_${TARGET//-/_}=$TC/bin/$CLANG"
export "AR_${TARGET//-/_}=$TC/bin/llvm-ar"

cd "$(dirname "$0")"   # src-tauri
cargo build --lib --release --target "$TARGET" --no-default-features

OUT="gen/android/app/src/main/jniLibs/$ABI"
mkdir -p "$OUT"
rm -f "$OUT/libsendsent_lib.so"
cp "target/$TARGET/release/libsendsent_lib.so" "$OUT/libsendsent_lib.so"
echo "built $OUT/libsendsent_lib.so"
