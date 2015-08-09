extern crate pkg_config;
extern crate semver;
extern crate cmake;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    register_dep("SSH2");
    register_dep("OPENSSL");
    let has_pkgconfig = Command::new("pkg-config").output().is_ok();

    // Try to find if a compatible ligbit2 is available, via pkg-config
    let git2_pc = pkg_config::find_library("libgit2");
    if git2_pc.is_ok() {
        let req_vers = semver::VersionReq::parse("0.23").unwrap();
        let git2_vers = semver::Version::parse(git2_pc.unwrap().version.as_ref());
        if git2_vers.is_ok() && req_vers.matches(&git2_vers.unwrap()) {
            return
        }
    }

    let target = env::var("TARGET").unwrap();
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
        cfg.define("STATIC_CRT", "OFF");
    }

    // libgit2 uses pkg-config to discover libssh2, but this doesn't work on
    // windows as libssh2 doesn't come with a libssh2.pc file in that install
    // (or when pkg-config isn't found). As a result we just manually turn on
    // SSH support in libgit2 (a little jankily) here...
    if windows || !has_pkgconfig {
        if msvc {
            cfg.cflag("/DGIT_SSH");
        } else {
            cfg.cflag("-DGIT_SSH");
        }
        if let Ok(libssh2_include) = env::var("DEP_SSH2_INCLUDE") {
            if msvc {
                cfg.cflag(format!(" /I{}", libssh2_include));
            } else {
                cfg.cflag(format!(" -I{}", libssh2_include));
            }
        }
    }

    let dst = cfg.define("BUILD_SHARED_LIBS", "OFF")
                 .define("BUILD_CLAR", "OFF")
                 .define("CURL", "OFF")
                 .register_dep("SSH2")
                 .register_dep("Z")
                 .register_dep("OPENSSL")
                 .build();

    let flags = dst.join("build/CMakeFiles/git2.dir/flags.make");
    let mut contents = String::new();

    // Make sure libssh2 was detected on unix systems, because it definitely
    // should have been!
    if !msvc {
        File::open(flags).unwrap().read_to_string(&mut contents).unwrap();
        if !contents.contains("-DGIT_SSH") {
            panic!("libgit2 failed to find libssh2, and SSH support is required");
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

    if env::var("HOST") == env::var("TARGET") {
        prepend("PKG_CONFIG_PATH", dst.join("lib/pkgconfig"));
        if pkg_config::Config::new().statik(true).find("libgit2").is_ok() {
            return
        }
    }

    println!("cargo:rustc-link-lib=static=git2");
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=iconv");
    }
}

fn register_dep(dep: &str) {
    match env::var(&format!("DEP_{}_ROOT", dep)) {
        Ok(s) => {
            prepend("PKG_CONFIG_PATH", Path::new(&s).join("lib/pkgconfig"));
        }
        Err(..) => {}
    }
}

fn prepend(var: &str, val: PathBuf) {
    let prefix = env::var(var).unwrap_or(String::new());
    let mut v = vec![val];
    v.extend(env::split_paths(&prefix));
    env::set_var(var, &env::join_paths(v).unwrap());
}
