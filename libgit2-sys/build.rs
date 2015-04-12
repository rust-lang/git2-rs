extern crate pkg_config;

use std::io::ErrorKind;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    register_dep("SSH2");
    register_dep("OPENSSL");

    let mut cflags = env::var("CFLAGS").unwrap_or(String::new());
    let target = env::var("TARGET").unwrap();
    let mingw = target.contains("windows-gnu");
    cflags.push_str(" -ffunction-sections -fdata-sections");

    if target.contains("i686") {
        cflags.push_str(" -m32");
    } else if target.contains("x86_64") {
        cflags.push_str(" -m64");
    }
    if !target.contains("i686") {
        cflags.push_str(" -fPIC");
    }

    let src = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let dst = PathBuf::from(&env::var("OUT_DIR").unwrap());
    let _ = fs::create_dir(&dst.join("build"));

    let mut cmd = Command::new("cmake");
    cmd.arg(&src.join("libgit2"))
       .current_dir(&dst.join("build"));
    if mingw {
        cmd.arg("-G").arg("Unix Makefiles");
    }
    let profile = match &env::var("PROFILE").unwrap()[..] {
        "bench" | "release" => "Release",
        _ => "Debug",
    };
    run(cmd.arg("-DTHREADSAFE=ON")
           .arg("-DBUILD_SHARED_LIBS=OFF")
           .arg("-DBUILD_CLAR=OFF")
           .arg(&format!("-DCMAKE_BUILD_TYPE={}", profile))
           .arg(&format!("-DCMAKE_INSTALL_PREFIX={}", dst.display()))
           .arg("-DBUILD_EXAMPLES=OFF")
           .arg(&format!("-DCMAKE_C_FLAGS={}", cflags)), "cmake");
    run(Command::new("cmake")
                .arg("--build").arg(".")
                .arg("--target").arg("install")
                .current_dir(&dst.join("build")), "cmake");

    println!("cargo:root={}", dst.display());
    if mingw || target.contains("windows") {
        println!("cargo:rustc-flags=-l winhttp -l rpcrt4 -l ole32 \
                                    -l ws2_32 -l bcrypt -l crypt32 \
                                    -l static=git2");
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        return
    }

    if env::var("HOST") == env::var("TARGET") {
        append("PKG_CONFIG_PATH", dst.join("lib/pkgconfig"));
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
    match cmd.output() {
        Ok(out) => {
            if !out.status.success() {
                fail(&format!("command did not execute successfully, got: {}\n --stdout: {}\n --stderr: {}\n"
                    , out.status
                    , String::from_utf8_lossy(&out.stdout)
                    , String::from_utf8_lossy(&out.stderr)));
            }
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            fail(&format!("failed to execute command: {}\nis `{}` not installed?",
                          e, program));
        }
        Err(e) => fail(&format!("failed to execute command: {}", e)),
    };
}

fn register_dep(dep: &str) {
    match env::var(&format!("DEP_{}_ROOT", dep)) {
        Ok(s) => {
            append("CMAKE_PREFIX_PATH", PathBuf::from(&s));
            append("PKG_CONFIG_PATH", Path::new(&s).join("lib/pkgconfig"));
        }
        Err(..) => {}
    }
}

fn append(var: &str, val: PathBuf) {
    let prefix = env::var(var).unwrap_or(String::new());
    let val = env::join_paths(env::split_paths(&prefix)
                                  .chain(Some(val).into_iter())).unwrap();
    env::set_var(var, &val);
}

fn fail(s: &str) -> ! {
    panic!("\n{}\n\nbuild script failed, must exit now", s)
}
