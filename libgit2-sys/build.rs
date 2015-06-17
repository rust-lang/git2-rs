extern crate pkg_config;

use std::env;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(n) => n,
        Err(e) => fail(&format!("\n{} failed with {}\n", stringify!($e), e)),
    })
}

fn main() {
    register_dep("SSH2");
    register_dep("OPENSSL");

    if env::var("LIBSSH2_SYS_USE_PKG_CONFIG").is_ok() {
        if pkg_config::find_library("libgit2").is_ok() {
            return
        }
    }

    let mut cflags = env::var("CFLAGS").unwrap_or(String::new());
    let target = env::var("TARGET").unwrap();
    let mingw = target.contains("windows-gnu");
    let msvc = target.contains("msvc");

    if msvc {
        // libgit2 passes the /GL flag to enable whole program optimization, but
        // this requires that the /LTCG flag is passed to the linker later on,
        // and currently the compiler does not do that, so we disable whole
        // program optimization entirely.
        cflags.push_str(" /GL-");
    } else {
        cflags.push_str(" -ffunction-sections -fdata-sections");

        if target.contains("i686") {
            cflags.push_str(" -m32");
        } else if target.contains("x86_64") {
            cflags.push_str(" -m64");
        }
        if !target.contains("i686") {
            cflags.push_str(" -fPIC");
        }
    }

    // libgit2 uses pkg-config to discover libssh2, but this doesn't work on
    // windows as libssh2 doesn't come with a libssh2.pc file in that install.
    // As a result we just manually turn on SSH support in libgit2 (a little
    // jankily) here...
    if mingw {
        cflags.push_str(" -DGIT_SSH");
        let libssh2_root = env::var("DEP_SSH2_ROOT").unwrap();
        cflags.push_str(&format!(" -I{}/include", libssh2_root));
    } else if msvc {
        cflags.push_str(" /DGIT_SSH");
        let libssh2_root = env::var("DEP_SSH2_ROOT").unwrap();
        cflags.push_str(&format!(" /I{}\\include", libssh2_root));
    }

    let src = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let dst = PathBuf::from(&env::var("OUT_DIR").unwrap());
    let _ = fs::create_dir(&dst.join("build"));

    let mut cmd = Command::new("cmake");
    cmd.arg(&src.join("libgit2"))
       .current_dir(&dst.join("build"));
    if mingw {
        cmd.arg("-G").arg("Unix Makefiles");
    } else if msvc {
        // If we don't pass this unfortunately cmake produces 32-bit builds
        cmd.arg("-G").arg("Visual Studio 12 2013 Win64");

        // Currently liblibc links to msvcrt which apparently is a dynamic CRT,
        // so we need to turn this off to get it to link right.
        cmd.arg("-DSTATIC_CRT=OFF");
    }
    let profile = match &env::var("PROFILE").unwrap()[..] {
        "bench" | "release" => "Release",
        _ if msvc => "Release", // currently we need to always use the same CRT
        _ => "Debug",
    };
    run(cmd.arg("-DBUILD_SHARED_LIBS=OFF")
           .arg("-DBUILD_CLAR=OFF")
           .arg(&format!("-DCMAKE_BUILD_TYPE={}", profile))
           .arg(&format!("-DCMAKE_INSTALL_PREFIX={}", dst.display()))
           .arg(&format!("-DCMAKE_C_FLAGS={}", cflags)), "cmake");

    let flags = dst.join("build/CMakeFiles/git2.dir/flags.make");
    let mut contents = String::new();

    // Make sure libssh2 was detected on unix systems, because it definitely
    // should have been!
    if !msvc {
        t!(t!(File::open(flags)).read_to_string(&mut contents));
        if !contents.contains("-DGIT_SSH") {
            fail("libgit2 failed to find libssh2, and SSH support is required");
        }
    }

    run(Command::new("cmake")
                .arg("--build").arg(".")
                .arg("--target").arg("install")
                .arg("--config").arg(profile)
                .current_dir(&dst.join("build")), "cmake");

    println!("cargo:root={}", dst.display());
    if mingw || target.contains("windows") {
        println!("cargo:rustc-flags=-l winhttp -l rpcrt4 -l ole32 \
                                    -l static=git2");
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        return
    }

    if env::var("HOST") == env::var("TARGET") {
        prepend("PKG_CONFIG_PATH", dst.join("lib/pkgconfig"));
        if pkg_config::Config::new().statik(true).find("libgit2").is_ok() {
            return
        }
    }

    println!("cargo:rustc-flags=-l static=git2");
    println!("cargo:rustc-flags=-L {}", dst.join("lib").display());
    if target.contains("apple") {
        println!("cargo:rustc-flags=-l iconv");
    }
}

fn run(cmd: &mut Command, program: &str) {
    println!("running: {:?}", cmd);
    let status = match cmd.status() {
        Ok(status) => status,
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            fail(&format!("failed to execute command: {}\nis `{}` not installed?",
                          e, program));
        }
        Err(e) => fail(&format!("failed to execute command: {}", e)),
    };
    if !status.success() {
        fail(&format!("command did not execute successfully, got: {}", status));
    }
}

fn register_dep(dep: &str) {
    match env::var(&format!("DEP_{}_ROOT", dep)) {
        Ok(s) => {
            prepend("CMAKE_PREFIX_PATH", PathBuf::from(&s));
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

fn fail(s: &str) -> ! {
    panic!("\n{}\n\nbuild script failed, must exit now", s)
}
