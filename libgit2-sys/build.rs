extern crate cc;
extern crate pkg_config;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let https = env::var("CARGO_FEATURE_HTTPS").is_ok();
    let ssh = env::var("CARGO_FEATURE_SSH").is_ok();
    let curl = env::var("CARGO_FEATURE_CURL").is_ok();

    if env::var("LIBGIT2_SYS_USE_PKG_CONFIG").is_ok() {
        if pkg_config::find_library("libgit2").is_ok() {
            return
        }
    }

    if !Path::new("libgit2/.git").exists() {
        let _ = Command::new("git").args(&["submodule", "update", "--init"])
                                   .status();
    }

    let target = env::var("TARGET").unwrap();
    let windows = target.contains("windows");
    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let include = dst.join("include");
    let mut cfg = cc::Build::new();
    fs::create_dir_all(&include).unwrap();

    // Copy over all header files
    cp_r("libgit2/include".as_ref(), &include);

    cfg.include(&include)
        .include("libgit2/src")
        .out_dir(dst.join("build"))
        .warnings(false);

    // Include all cross-platform C files
    add_c_files(&mut cfg, "libgit2/src".as_ref());
    add_c_files(&mut cfg, "libgit2/src/xdiff".as_ref());

    // These are activated by feautres, but they're all unconditionally always
    // compiled apparently and have internal #define's to make sure they're
    // compiled correctly.
    add_c_files(&mut cfg, "libgit2/src/transports".as_ref());
    add_c_files(&mut cfg, "libgit2/src/streams".as_ref());

    // Always use bundled http-parser for now
    cfg.include("libgit2/deps/http-parser")
        .file("libgit2/deps/http-parser/http_parser.c");

    // Always use bundled regex for now
    cfg.include("libgit2/deps/regex")
        .file("libgit2/deps/regex/regex.c");

    if windows {
        add_c_files(&mut cfg, "libgit2/src/win32".as_ref());
        cfg.define("STRSAFE_NO_DEPRECATE", None);

        // libgit2's build system claims that forks like mingw-w64 of MinGW
        // still want this define to use C99 stdio functions automatically.
        // Apparently libgit2 breaks at runtime if this isn't here? Who knows!
        if target.contains("gnu") {
            cfg.define("__USE_MINGW_ANSI_STDIO", "1");
        }
    } else {
        add_c_files(&mut cfg, "libgit2/src/unix".as_ref());
        cfg.flag("-fvisibility=hidden");
    }
    if target.contains("solaris") {
        cfg.define("_POSIX_C_SOURCE", "200112L");
        cfg.define("__EXTENSIONS__", None);
    }

    let mut features = String::new();

    features.push_str("#ifndef INCLUDE_features_h\n");
    features.push_str("#define INCLUDE_features_h\n");
    features.push_str("#define GIT_THREADS 1\n");

    if !target.contains("android") {
        features.push_str("#define GIT_USE_NSEC 1\n");
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
        features.push_str("#define GIT_SSH_MEMORY_CREDENTIALS 1\n");
    }
    if https {
        features.push_str("#define GIT_HTTPS 1\n");

        if windows {
            features.push_str("#define GIT_WINHTTP 1\n");
            features.push_str("#define GIT_SHA1_WIN32 1\n");
            cfg.file("libgit2/src/hash/hash_win32.c");
        } else if target.contains("apple") {
            features.push_str("#define GIT_SECURE_TRANSPORT 1\n");
            features.push_str("#define GIT_SHA1_COMMON_CRYPTO 1\n");
        } else {
            features.push_str("#define GIT_OPENSSL 1\n");
            features.push_str("#define GIT_SHA1_OPENSSL 1\n");
            if let Some(path) = env::var_os("DEP_OPENSSL_INCLUDE") {
                cfg.include(path);
            }
        }
    } else {
        cfg.file("libgit2/src/hash/hash_generic.c");
    }

    if curl {
        features.push_str("#define GIT_CURL 1\n");
        if let Some(path) = env::var_os("DEP_CURL_INCLUDE") {
            cfg.include(path);
        }
        // Handle dllimport/dllexport on windows by making sure that if we built
        // curl statically (as told to us by the `curl-sys` crate) we define the
        // correct values for curl's header files.
        if env::var_os("DEP_CURL_STATIC").is_some() {
            cfg.define("CURL_STATICLIB", None);
        }
    }
    if let Some(path) = env::var_os("DEP_Z_INCLUDE") {
        cfg.include(path);
    }

    if target.contains("apple") {
        features.push_str("#define GIT_USE_ICONV 1\n");
    }

    features.push_str("#endif\n");
    fs::write(include.join("git2/sys/features.h"), features).unwrap();

    cfg.compile("git2");

    println!("cargo:root={}", dst.display());

    if target.contains("windows") {
        println!("cargo:rustc-link-lib=winhttp");
        println!("cargo:rustc-link-lib=rpcrt4");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=crypt32");
        return
    }

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}

fn cp_r(from: &Path, to: &Path) {
    for e in from.read_dir().unwrap() {
        let e = e.unwrap();
        let from = e.path();
        let to = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            fs::create_dir_all(&to).unwrap();
            cp_r(&from, &to);
        } else {
            println!("{} => {}", from.display(), to.display());
            fs::copy(&from, &to).unwrap();
        }
    }
}

fn add_c_files(build: &mut cc::Build, path: &Path) {
    for e in path.read_dir().unwrap() {
        let e = e.unwrap();
        let path = e.path();
        if e.file_type().unwrap().is_dir() {
            // skip dirs for now
        } else if path.extension().and_then(|s| s.to_str()) == Some("c") {
            build.file(&path);
        }
    }
}
