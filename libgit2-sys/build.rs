use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let https = env::var("CARGO_FEATURE_HTTPS").is_ok();
    let ssh = env::var("CARGO_FEATURE_SSH").is_ok();

    let mut cfg = pkg_config::Config::new();
    if let Ok(lib) = cfg.atleast_version("1.0.0").probe("libgit2") {
        for include in &lib.include_paths {
            println!("cargo:root={}", include.display());
        }
        return;
    }

    if !Path::new("libgit2/.git").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init"])
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
        .include("libgit2/src")
        .out_dir(dst.join("build"))
        .warnings(false);

    // Include all cross-platform C files
    add_c_files(&mut cfg, "libgit2/src");
    add_c_files(&mut cfg, "libgit2/src/xdiff");

    // These are activated by features, but they're all unconditionally always
    // compiled apparently and have internal #define's to make sure they're
    // compiled correctly.
    add_c_files(&mut cfg, "libgit2/src/transports");
    add_c_files(&mut cfg, "libgit2/src/streams");

    // Always use bundled http-parser for now
    cfg.include("libgit2/deps/http-parser")
        .file("libgit2/deps/http-parser/http_parser.c");

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

    cfg.file("libgit2/src/allocators/stdalloc.c");

    if windows {
        add_c_files(&mut cfg, "libgit2/src/win32");
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
        add_c_files(&mut cfg, "libgit2/src/unix");
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
    cfg.file("libgit2/src/hash/sha1/collisiondetect.c");
    cfg.file("libgit2/src/hash/sha1/sha1dc/sha1.c");
    cfg.file("libgit2/src/hash/sha1/sha1dc/ubc_check.c");

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
        return;
    }

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
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
    for e in path.as_ref().read_dir().unwrap() {
        let e = e.unwrap();
        let path = e.path();
        if e.file_type().unwrap().is_dir() {
            // skip dirs for now
        } else if path.extension().and_then(|s| s.to_str()) == Some("c") {
            build.file(&path);
        }
    }
}
