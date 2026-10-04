#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SDK=${ANDROID_HOME:-"$HOME/Library/Android/sdk"}
NDK=${ANDROID_NDK_ROOT:-${ANDROID_NDK_HOME:-"$SDK/ndk/29.0.14206865"}}
case "$(uname -s)" in
  Darwin) HOST_TAG=darwin-x86_64 ;;
  Linux) HOST_TAG=linux-x86_64 ;;
  *) echo "Unsupported build host" >&2; exit 1 ;;
esac
TC="$NDK/toolchains/llvm/prebuilt/$HOST_TAG"
export ANDROID_NDK_ROOT="$NDK" ANDROID_PLATFORM=android-28
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target/android"}
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TC/bin/aarch64-linux-android28-clang"
export CC_aarch64_linux_android="$TC/bin/aarch64-linux-android28-clang"
export CXX_aarch64_linux_android="$TC/bin/aarch64-linux-android28-clang++"
export AR_aarch64_linux_android="$TC/bin/llvm-ar"
export CARGO_ENCODED_RUSTFLAGS=$'-C\x1ftarget-cpu=generic\x1f-C\x1flink-arg=-Wl,-z,max-page-size=16384'
cd "$ROOT"
cargo build --release --locked --target aarch64-linux-android --features load-dynamic --example android_load
cargo test --release --locked --target aarch64-linux-android -p knf-rs --no-run
"$TC/bin/aarch64-linux-android28-clang" "$ROOT/scripts/android_loader.c" -ldl \
  -Wl,-z,max-page-size=16384 -o "$CARGO_TARGET_DIR/android_loader"
echo "Output: $CARGO_TARGET_DIR/aarch64-linux-android/release/examples/libandroid_load.so"
