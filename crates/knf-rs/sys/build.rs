use cmake::Config;
use std::env;
use std::path::{Path, PathBuf};

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if std::env::var("BUILD_DEBUG").is_ok() {
            println!("cargo:warning=[DEBUG] {}", format!($($arg)*));
        }
    };
}

fn copy_folder(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("Failed to create dst directory");
    if cfg!(unix) {
        std::process::Command::new("cp")
            .arg("-rf")
            .arg(src)
            .arg(dst.parent().expect("no parent"))
            .status()
            .expect("Failed to execute cp command");
    }

    if cfg!(windows) {
        std::process::Command::new("robocopy.exe")
            .arg("/e")
            .arg(src)
            .arg(dst)
            .status()
            .expect("Failed to execute robocopy command");
    }
}

fn main() {
    let target = env::var("TARGET").expect("no target");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("no out dir"));
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let knf_src = Path::new(&manifest_dir).join("knf");
    let knf_dst = out_dir.join("knf");
    let knfc_src = Path::new(&manifest_dir).join("knfc");
    let knfc_dst = out_dir.join("knfc");
    let static_crt = env::var("KNF_STATIC_CRT")
        .map(|v| v == "1")
        .unwrap_or(false);

    let profile = if env::var("PROFILE").as_deref() == Ok("debug") {
        "Debug"
    } else {
        "Release"
    };
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("no target OS");
    let android = if target_os == "android" {
        let ndk = env::var_os("ANDROID_NDK_ROOT")
            .or_else(|| env::var_os("ANDROID_NDK_HOME"))
            .or_else(|| env::var_os("ANDROID_NDK"))
            .map(PathBuf::from)
            .expect("Android build requires ANDROID_NDK_ROOT or ANDROID_NDK_HOME");
        let host = env::var("HOST").expect("no host");
        let host_tag = if host.contains("apple") {
            "darwin-x86_64"
        } else if host.contains("windows") {
            "windows-x86_64"
        } else {
            "linux-x86_64"
        };
        let sysroot = ndk
            .join("toolchains/llvm/prebuilt")
            .join(host_tag)
            .join("sysroot");
        let (abi, lib_target) = match target.as_str() {
            "aarch64-linux-android" => ("arm64-v8a", "aarch64-linux-android"),
            "armv7-linux-androideabi" => ("armeabi-v7a", "arm-linux-androideabi"),
            "x86_64-linux-android" => ("x86_64", "x86_64-linux-android"),
            "i686-linux-android" => ("x86", "i686-linux-android"),
            _ => panic!("Unsupported Android target: {target}"),
        };
        let api = env::var("ANDROID_PLATFORM").unwrap_or_else(|_| "android-21".into());
        Some((ndk, sysroot, abi, lib_target, api))
    } else {
        None
    };

    debug_log!("TARGET: {}", target);
    debug_log!("CARGO_MANIFEST_DIR: {}", manifest_dir);
    debug_log!("OUT_DIR: {}", out_dir.display());

    if !knf_dst.exists() {
        debug_log!("Copy {} to {}", knf_src.display(), knf_dst.display());
        copy_folder(&knf_src, &knf_dst);
    }

    if !knfc_dst.exists() {
        debug_log!("Copy {} to {}", knfc_src.display(), knfc_dst.display());
        copy_folder(&knfc_src, &knfc_dst);
    }

    // Bindings
    let mut bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .clang_arg(format!("-I{}", knfc_dst.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    if let Some((_, sysroot, _, _, api)) = &android {
        let api = api.strip_prefix("android-").unwrap_or(api);
        bindings = bindings
            .clang_arg(format!("--target={target}{api}"))
            .clang_arg(format!("--sysroot={}", sysroot.display()))
            .clang_arg(format!(
                "-isystem{}",
                sysroot.join("usr/include/c++/v1").display()
            ));
    }
    let bindings = bindings.generate().expect("Failed to generate bindings");

    // Write the generated bindings to an output file
    let bindings_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(bindings_path)
        .expect("Failed to write bindings");

    println!("cargo:rerun-if-changed=./knf");
    println!("cargo:rerun-if-changed=./knfc");
    println!("cargo:rerun-if-changed=wrapper.hpp");

    debug_log!("Bindings Created");

    let mut config = Config::new(&knfc_dst);

    if let Some((ndk, _, abi, _, api)) = &android {
        config
            .define(
                "CMAKE_TOOLCHAIN_FILE",
                ndk.join("build/cmake/android.toolchain.cmake"),
            )
            .define("ANDROID_ABI", abi)
            .define("ANDROID_PLATFORM", api)
            .define("ANDROID_STL", "c++_static");
    }

    if target_os == "windows" {
        config.static_crt(static_crt);
        debug_log!("STATIC_CRT: {}", static_crt);
    }

    config
        .profile(profile)
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        .very_verbose(std::env::var("CMAKE_VERBOSE").is_ok()) // Not verbose by default
        .always_configure(false)
        .build();

    println!("cargo:rustc-link-search={}", out_dir.join("lib").display());

    // Link
    if target_os == "macos" || target_os == "ios" {
        println!("cargo:rustc-link-lib=c++");
    }

    if target_os == "linux" {
        println!("cargo:rustc-link-lib=stdc++");
    }

    if let Some((_, sysroot, _, lib_target, _)) = &android {
        // Do not add the NDK system library directory to Rust's search path:
        // it also contains libc.a and can accidentally statically link Bionic.
        for library in ["libc++_static.a", "libc++abi.a"] {
            std::fs::copy(
                sysroot.join("usr/lib").join(lib_target).join(library),
                out_dir.join("lib").join(library),
            )
            .expect("Failed to copy NDK C++ runtime");
        }
        println!("cargo:rustc-link-lib=static=c++_static");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
    for variable in [
        "ANDROID_NDK_ROOT",
        "ANDROID_NDK_HOME",
        "ANDROID_NDK",
        "ANDROID_PLATFORM",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    println!("cargo:rustc-link-lib=static=knfc");
    println!("cargo:rustc-link-lib=static=kaldi-native-fbank-core");
}
