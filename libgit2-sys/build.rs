use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Tries to use system libgit2 and emits necessary build script instructions.
fn try_system_libgit2(
    experimental_sha256: bool,
) -> Result<pkg_config::Library, Box<dyn std::error::Error>> {
    let mut cfg = pkg_config::Config::new();
    let range_version = "1.9.2".."1.10.0";

    let lib = if experimental_sha256 {
        // Determine whether experimental SHA256 object support is enabled.
        //
        // Given the SHA256 support is ABI-incompatible,
        // we take a conservative approach here:
        //
        // 1. Only accept if the library name has the `-experimental` suffix
        // 2. The experimental library must have an `experimental.h` header file
        //    containing an enabled `GIT_EXPERIMENTAL_SHA256` constant.
        //
        // See how libgit2 handles experimental features:
        // https://github.com/libgit2/libgit2/blob/3ac4c0adb1064bad16a7f980d87e7261753fd07e/cmake/ExperimentalFeatures.cmake
        match cfg
            .range_version(range_version.clone())
            .probe("libgit2-experimental")
        {
            Ok(lib) => {
                let sha256_constant_in_header = lib.include_paths.iter().any(|path| {
                    let header = path.join("git2/experimental.h");
                    let contents = match std::fs::read_to_string(header) {
                        Ok(s) => s,
                        Err(_) => return false,
                    };
                    if contents.contains("#cmakedefine") {
                        // still template
                        return false;
                    }
                    contents
                        .lines()
                        .any(|l| l.starts_with("#define GIT_EXPERIMENTAL_SHA256 1"))
                });
                if sha256_constant_in_header {
                    println!("cargo:rustc-cfg=libgit2_experimental_sha256");
                    lib
                } else {
                    println!("cargo:warning=no GIT_EXPERIMENTAL_SHA256 constant in headers");
                    return Err(
                        "GIT_EXPERIMENTAL_SHA256 wasn't enabled for libgit2-experimental library"
                            .into(),
                    );
                }
            }
            Err(e) => {
                println!("cargo:warning=failed to probe system libgit2-experimental: {e}");
                return Err(e.into());
            }
        }
    } else {
        match cfg.range_version(range_version).probe("libgit2") {
            Ok(lib) => lib,
            Err(e) => {
                println!("cargo:warning=failed to probe system libgit2: {e}");
                return Err(e.into());
            }
        }
    };

    for include in &lib.include_paths {
        println!("cargo:root={}", include.display());
    }
    Ok(lib)
}

fn main() {
    println!(
        "cargo:rustc-check-cfg=cfg(\
            libgit2_vendored,\
            libgit2_experimental_sha256,\
        )"
    );

    let https = env::var("CARGO_FEATURE_HTTPS").is_ok();
    let ssh = env::var("CARGO_FEATURE_SSH").is_ok();
    let vendored = env::var("CARGO_FEATURE_VENDORED").is_ok();
    let zlib_ng_compat = env::var("CARGO_FEATURE_ZLIB_NG_COMPAT").is_ok();
    let unstable_sha256 = env::var("CARGO_FEATURE_UNSTABLE_SHA256").is_ok();

    // Specify `LIBGIT2_NO_VENDOR` to force to use system libgit2.
    // Due to the additive nature of Cargo features, if some crate in the
    // dependency graph activates `vendored` feature, there is no way to revert
    // it back. This env var serves as a workaround for this purpose.
    println!("cargo:rerun-if-env-changed=LIBGIT2_NO_VENDOR");
    let forced_no_vendor = env::var_os("LIBGIT2_NO_VENDOR").map_or(false, |s| s != "0");

    if forced_no_vendor {
        if try_system_libgit2(unstable_sha256).is_err() {
            panic!(
                "\
The environment variable `LIBGIT2_NO_VENDOR` has been set but no compatible system libgit2 could be found.
The build is now aborting. To disable, unset the variable or use `LIBGIT2_NO_VENDOR=0`.
",
            );
        }

        // We've reached here, implying we're using system libgit2.
        return;
    }

    // To use zlib-ng in zlib-compat mode, we have to build libgit2 ourselves.
    let try_to_use_system_libgit2 = !vendored && !zlib_ng_compat;
    if try_to_use_system_libgit2 && try_system_libgit2(unstable_sha256).is_ok() {
        // using system libgit2 has worked
        return;
    }

    println!("cargo:rustc-cfg=libgit2_vendored");

    if !Path::new("libgit2/src").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init", "libgit2"])
            .status();
    }

    let target = env::var("TARGET").unwrap();
    let windows = target.contains("windows");
    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let include = dst.join("include");
    let mut cfg = cc::Build::new();
    fs::create_dir_all(&include).unwrap();

    // Copy over all header files
    cp_r("libgit2/include", &include);

    cfg.include(&include)
        .include("libgit2/src/libgit2")
        .include("libgit2/src/util")
        .out_dir(dst.join("build"))
        .warnings(false);

    if unstable_sha256 {
        println!("cargo:rustc-cfg=libgit2_experimental_sha256");
        cfg.define("GIT_EXPERIMENTAL_SHA256", "1");
    }

    // Include all cross-platform C files
    add_c_files(&mut cfg, "libgit2/src/libgit2");
    add_c_files(&mut cfg, "libgit2/src/util");

    // These are activated by features, but they're all unconditionally always
    // compiled apparently and have internal #define's to make sure they're
    // compiled correctly.
    add_c_files(&mut cfg, "libgit2/src/libgit2/transports");
    add_c_files(&mut cfg, "libgit2/src/libgit2/streams");

    // Always use bundled HTTP parser (llhttp) for now
    cfg.include("libgit2/deps/llhttp");
    add_c_files(&mut cfg, "libgit2/deps/llhttp");

    // external/system xdiff is not yet supported
    cfg.include("libgit2/deps/xdiff");
    add_c_files(&mut cfg, "libgit2/deps/xdiff");

    // Use the included PCRE regex backend.
    //
    // Ideally these defines would be specific to the pcre files (or placed in
    // a config.h), but since libgit2 already has a config.h used for other
    // reasons, just define on the command-line for everything. Perhaps there
    // is some way with cc to have different instructions per-file?
    cfg.define("GIT_REGEX_BUILTIN", "1")
        .include("libgit2/deps/pcre")
        .define("HAVE_STDINT_H", Some("1"))
        .define("HAVE_MEMMOVE", Some("1"))
        .define("NO_RECURSE", Some("1"))
        .define("NEWLINE", Some("10"))
        .define("POSIX_MALLOC_THRESHOLD", Some("10"))
        .define("LINK_SIZE", Some("2"))
        .define("PARENS_NEST_LIMIT", Some("250"))
        .define("MATCH_LIMIT", Some("10000000"))
        .define("MATCH_LIMIT_RECURSION", Some("MATCH_LIMIT"))
        .define("MAX_NAME_SIZE", Some("32"))
        .define("MAX_NAME_COUNT", Some("10000"));
    // "no symbols" warning on pcre_string_utils.c is because it is only used
    // when when COMPILE_PCRE8 is not defined, which is the default.
    add_c_files(&mut cfg, "libgit2/deps/pcre");

    cfg.file("libgit2/src/util/allocators/failalloc.c");
    cfg.file("libgit2/src/util/allocators/stdalloc.c");

    if windows {
        add_c_files(&mut cfg, "libgit2/src/util/win32");
        cfg.define("STRSAFE_NO_DEPRECATE", None);
        cfg.define("WIN32", None);
        cfg.define("_WIN32_WINNT", Some("0x0600"));

        // libgit2's build system claims that forks like mingw-w64 of MinGW
        // still want this define to use C99 stdio functions automatically.
        // Apparently libgit2 breaks at runtime if this isn't here? Who knows!
        if target.contains("gnu") {
            cfg.define("__USE_MINGW_ANSI_STDIO", "1");
        }
    } else {
        add_c_files(&mut cfg, "libgit2/src/util/unix");
        cfg.flag("-fvisibility=hidden");
    }
    if target.contains("solaris") || target.contains("illumos") {
        cfg.define("_POSIX_C_SOURCE", "200112L");
        cfg.define("__EXTENSIONS__", None);
    }

    let mut features = String::new();

    features.push_str("#ifndef INCLUDE_features_h\n");
    features.push_str("#define INCLUDE_features_h\n");
    features.push_str("#define GIT_THREADS 1\n");
    features.push_str("#define GIT_TRACE 1\n");
    features.push_str("#define GIT_HTTPPARSER_BUILTIN 1\n");

    if !target.contains("android") {
        features.push_str("#define GIT_USE_NSEC 1\n");
    }

    if windows {
        features.push_str("#define GIT_IO_WSAPOLL 1\n");
    } else {
        // Should we fallback to `select` as more systems have that?
        features.push_str("#define GIT_IO_POLL 1\n");
        features.push_str("#define GIT_IO_SELECT 1\n");
    }

    if target.contains("apple") {
        features.push_str("#define GIT_USE_STAT_MTIMESPEC 1\n");
    } else {
        features.push_str("#define GIT_USE_STAT_MTIM 1\n");
    }

    if env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() == "32" {
        features.push_str("#define GIT_ARCH_32 1\n");
    } else {
        features.push_str("#define GIT_ARCH_64 1\n");
    }

    if ssh {
        if let Some(path) = env::var_os("DEP_SSH2_INCLUDE") {
            cfg.include(path);
        }
        features.push_str("#define GIT_SSH 1\n");
        features.push_str("#define GIT_SSH_LIBSSH2 1\n");
        features.push_str("#define GIT_SSH_LIBSSH2_MEMORY_CREDENTIALS 1\n");
    }
    if https {
        features.push_str("#define GIT_HTTPS 1\n");

        if windows {
            features.push_str("#define GIT_WINHTTP 1\n");
        } else if target.contains("apple") {
            features.push_str("#define GIT_SECURE_TRANSPORT 1\n");
        } else {
            features.push_str("#define GIT_OPENSSL 1\n");
            if let Some(path) = env::var_os("DEP_OPENSSL_INCLUDE") {
                cfg.include(path);
            }
        }
    }

    // Use the CollisionDetection SHA1 implementation.
    features.push_str("#define GIT_SHA1_COLLISIONDETECT 1\n");
    cfg.define("SHA1DC_NO_STANDARD_INCLUDES", "1");
    cfg.define("SHA1DC_CUSTOM_INCLUDE_SHA1_C", "\"common.h\"");
    cfg.define("SHA1DC_CUSTOM_INCLUDE_UBC_CHECK_C", "\"common.h\"");
    cfg.file("libgit2/src/util/hash/collisiondetect.c");
    cfg.file("libgit2/src/util/hash/sha1dc/sha1.c");
    cfg.file("libgit2/src/util/hash/sha1dc/ubc_check.c");

    if https {
        if windows {
            features.push_str("#define GIT_SHA256_WIN32 1\n");
            cfg.file("libgit2/src/util/hash/win32.c");
        } else if target.contains("apple") {
            features.push_str("#define GIT_SHA256_COMMON_CRYPTO 1\n");
            cfg.file("libgit2/src/util/hash/common_crypto.c");
        } else {
            features.push_str("#define GIT_SHA256_OPENSSL 1\n");
            cfg.file("libgit2/src/util/hash/openssl.c");
        }
    } else {
        features.push_str("#define GIT_SHA256_BUILTIN 1\n");
        cfg.file("libgit2/src/util/hash/builtin.c");
        cfg.file("libgit2/src/util/hash/rfc6234/sha224-256.c");
    }

    if let Some(path) = env::var_os("DEP_Z_INCLUDE") {
        cfg.include(path);
    }

    if target.contains("apple") {
        features.push_str("#define GIT_USE_ICONV 1\n");
    }

    features.push_str("#endif\n");
    fs::write(include.join("git2_features.h"), features).unwrap();

    cfg.compile("git2");

    println!("cargo:root={}", dst.display());

    if target.contains("windows") {
        println!("cargo:rustc-link-lib=winhttp");
        println!("cargo:rustc-link-lib=rpcrt4");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=secur32");
        println!("cargo:rustc-link-lib=advapi32");
    }

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }

    println!("cargo:rerun-if-changed=libgit2/include");
    println!("cargo:rerun-if-changed=libgit2/src");
    println!("cargo:rerun-if-changed=libgit2/deps");
}

fn cp_r(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    for e in from.as_ref().read_dir().unwrap() {
        let e = e.unwrap();
        let from = e.path();
        let to = to.as_ref().join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            fs::create_dir_all(&to).unwrap();
            cp_r(&from, &to);
        } else {
            println!("{} => {}", from.display(), to.display());
            fs::copy(&from, &to).unwrap();
        }
    }
}

fn add_c_files(build: &mut cc::Build, path: impl AsRef<Path>) {
    let path = path.as_ref();
    if !path.exists() {
        panic!("Path {} does not exist", path.display());
    }
    // sort the C files to ensure a deterministic build for reproducible builds
    let dir = path.read_dir().unwrap();
    let mut paths = dir.collect::<io::Result<Vec<_>>>().unwrap();
    paths.sort_by_key(|e| e.path());

    for e in paths {
        let path = e.path();
        if e.file_type().unwrap().is_dir() {
            // skip dirs for now
        } else if path.extension().and_then(|s| s.to_str()) == Some("c") {
            build.file(&path);
        }
    }
}
