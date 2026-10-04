# Android build and load validation

Use `scripts/build-android.sh` with `ANDROID_NDK_ROOT` (or
`ANDROID_NDK_HOME`) pointing to an installed NDK. The default SDK location is
`~/Library/Android/sdk`, NDK 29, ARM64, API 28. Output is isolated in
`target/android`; override `CARGO_TARGET_DIR` if needed. No tools are installed.

Dependencies are pinned to `ort =2.0.0-rc.13` and `ndarray =0.17.2`.
`load-dynamic` requires an application-provided ONNX Runtime shared library;
the validation used official ONNX Runtime Android 1.30.0.

The KNF build selects Android ABI from Cargo's target and uses the NDK
toolchain for both CMake and bindgen. C++ runtime archives are linked statically
without exposing the NDK's system library directory to the Rust linker.

## Reproduce

After building, copy these files to an ARM64 emulator or device:

- `target/android/aarch64-linux-android/release/examples/libandroid_load.so`
- `target/android/android_loader`
- An ARM64 `libonnxruntime.so` and an ONNX model, for the optional ORT check.

Run on the device (paths below assume `/data/local/tmp/pyannote`):

```sh
chmod 755 /data/local/tmp/pyannote/android_loader
# dlopen, KNF computation, dlclose, and normal process exit:
/data/local/tmp/pyannote/android_loader /data/local/tmp/pyannote/libandroid_load.so
# Additionally load an ONNX model and drop its session:
/data/local/tmp/pyannote/android_loader /data/local/tmp/pyannote/libandroid_load.so \
  /data/local/tmp/pyannote/libonnxruntime.so /data/local/tmp/pyannote/model.onnx
```

The ORT check retains the global environment and calls `_Exit` after flushing
the result. It checks library/model loading and session disposal, **not** normal
ORT process shutdown or JNI unloading. The KNF-only check exits normally.

## Local evidence, 2026-10-04

Before the fix, an Android rlib build succeeded, but the final KNF test link
failed: the NDK CMake toolchain produced ELF32 ARM objects while Rust targeted
AArch64. The absent `ANDROID_ABI` let the NDK select its default ABI.

After the fix, ARM64 shared-library linking and KNF test-executable linking
passed. On the Android 16 / API 36.1 ARM64 emulator, KNF produced 98 frames
with 80 finite feature bins. Native libraries use 16 KiB LOAD alignment; the
emulator itself has 4 KiB pages, so this is not a 16 KiB kernel execution test.

The current Apple-hosted emulator reports SME capability but faults when the
official ORT runtime executes that instruction. Without a diagnostic CPU
capability mask, the ORT check fails with SIGILL. With a test-only mask supplied
by the embedding project's diagnostics, the English G2P encoder ONNX model
loaded with two inputs and one output. This does not validate speaker embedding
inference or phone performance. No ORT source modification is included here.
