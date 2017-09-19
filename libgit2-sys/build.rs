extern crate cmake;
extern crate cc;
extern crate pkg_config;

use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! t {
    ($e:expr) => (match $e{
        Ok(e) => e,
        Err(e) => panic!("{} failed with {}", stringify!($e), e),
    })
}

fn main() {
    let https = env::var("CARGO_FEATURE_HTTPS").is_ok();
    let ssh = env::var("CARGO_FEATURE_SSH").is_ok();
    let curl = env::var("CARGO_FEATURE_CURL").is_ok();
    if ssh {
        register_dep("SSH2");
    }
    if https {
        register_dep("OPENSSL");
    }
    if curl {
        register_dep("CURL");
    }
    let has_pkgconfig = Command::new("pkg-config").output().is_ok();

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
    let host = env::var("HOST").unwrap();
    let windows = target.contains("windows");
    let msvc = target.contains("msvc");
    let mut cfg = cmake::Config::new("libgit2");

    if msvc {
        // libgit2 passes the /GL flag to enable whole program optimization, but
        // this requires that the /LTCG flag is passed to the linker later on,
        // and currently the compiler does not do that, so we disable whole
        // program optimization entirely.
        cfg.cflag("/GL-");

        // Currently liblibc links to msvcrt which apparently is a dynamic CRT,
        // so we need to turn this off to get it to link right.
        let features = env::var("CARGO_CFG_TARGET_FEATURE")
                          .unwrap_or(String::new());
        if features.contains("crt-static") {
            cfg.define("STATIC_CRT", "ON");
        } else {
            cfg.define("STATIC_CRT", "OFF");
        }
    }

    // libgit2 uses pkg-config to discover libssh2, but this doesn't work on
    // windows as libssh2 doesn't come with a libssh2.pc file in that install
    // (or when pkg-config isn't found). As a result we just manually turn on
    // SSH support in libgit2 (a little jankily) here...
    if ssh && (windows || !has_pkgconfig) {
        if let Ok(libssh2_include) = env::var("DEP_SSH2_INCLUDE") {
            if msvc {
                cfg.cflag(format!("/I{}", libssh2_include))
                   .cflag("/DGIT_SSH");
            } else {
                cfg.cflag(format!("-I{}", libssh2_include))
                   .cflag("-DGIT_SSH");
            }
        }
    }

    // When cross-compiling, we're pretty unlikely to find a `dlltool` binary
    // lying around, so try to find another if it exists
    if windows && !host.contains("windows") {
        let c_compiler = cc::Build::new().cargo_metadata(false)
                                           .get_compiler();
        let exe = c_compiler.path();
        let path = env::var_os("PATH").unwrap_or(OsString::new());
        let exe = env::split_paths(&path)
                      .map(|p| p.join(&exe))
                      .find(|p| p.exists());
        if let Some(exe) = exe {
            if let Some(name) = exe.file_name().and_then(|e| e.to_str()) {
                let name = name.replace("gcc", "dlltool");
                let dlltool = exe.with_file_name(name);
                cfg.define("DLLTOOL", &dlltool);
            }
        }
    }

    if ssh {
        cfg.register_dep("SSH2");
    } else {
        cfg.define("USE_SSH", "OFF");
    }
    if https {
        cfg.register_dep("OPENSSL");
    } else {
        cfg.define("USE_OPENSSL", "OFF");
    }
    if curl {
        cfg.register_dep("CURL");
    } else {
        cfg.define("CURL", "OFF");
    }

    let _ = fs::remove_dir_all(env::var("OUT_DIR").unwrap());
    t!(fs::create_dir_all(env::var("OUT_DIR").unwrap()));

    // Unset DESTDIR or libgit2.a ends up in it and cargo can't find it
    env::remove_var("DESTDIR");
    let dst = cfg.define("BUILD_SHARED_LIBS", "OFF")
                 .define("BUILD_CLAR", "OFF")
                 .register_dep("Z")
                 .build();

    // Make sure libssh2 was detected on unix systems, because it definitely
    // should have been!
    if ssh && !msvc {
        let flags = dst.join("build/CMakeFiles/git2.dir/flags.make");
        let mut contents = String::new();
        t!(t!(File::open(flags)).read_to_string(&mut contents));
        if !contents.contains("-DGIT_SSH") {
            panic!("libgit2 failed to find libssh2, and SSH support is required");
        }
    }

    // libgit2 requires the http_parser library for the HTTP transport to be
    // implemented, and it will attempt to use the system http_parser if it's
    // available. Detect this situation and report using the system http parser
    // the same way in this situation.
    //
    // Note that other dependencies of libgit2 like openssl, libz, and libssh2
    // are tracked via crates instead of this. Ideally this should be a crate as
    // well.
    let pkgconfig_file = dst.join("lib/pkgconfig/libgit2.pc");
    if let Ok(mut f) = File::open(&pkgconfig_file) {
        let mut contents = String::new();
        t!(f.read_to_string(&mut contents));
        if contents.contains("-lhttp_parser") {
            println!("cargo:rustc-link-lib=http_parser");
        }
    }

    if target.contains("windows") {
        println!("cargo:rustc-link-lib=winhttp");
        println!("cargo:rustc-link-lib=rpcrt4");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=static=git2");
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        return
    }

    println!("cargo:rustc-link-lib=static=git2");
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}

fn register_dep(dep: &str) {
    if let Some(s) = env::var_os(&format!("DEP_{}_ROOT", dep)) {
        prepend("PKG_CONFIG_PATH", Path::new(&s).join("lib/pkgconfig"));
        return
    }
    if let Some(s) = env::var_os(&format!("DEP_{}_INCLUDE", dep)) {
        let root = Path::new(&s).parent().unwrap();
        env::set_var(&format!("DEP_{}_ROOT", dep), root);
        let path = root.join("lib/pkgconfig");
        if path.exists() {
            prepend("PKG_CONFIG_PATH", path);
            return
        }
    }
}

fn prepend(var: &str, val: PathBuf) {
    let prefix = env::var(var).unwrap_or(String::new());
    let mut v = vec![val];
    v.extend(env::split_paths(&prefix));
    env::set_var(var, &env::join_paths(v).unwrap());
}
