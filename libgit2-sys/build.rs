extern crate "pkg-config" as pkg_config;

use std::os;
use std::io::{mod, fs, Command};
use std::io::process::InheritFd;

fn main() {
    register_dep("SSH2");
    register_dep("OPENSSL");

    let mut opts = pkg_config::default_options("libgit2");
    opts.atleast_version = Some("0.21.0".to_string());
    match pkg_config::find_library_opts("libgit2", &opts) {
        Ok(()) => return,
        Err(..) => {}
    }

    let mut cflags = os::getenv("CFLAGS").unwrap_or(String::new());
    let target = os::getenv("TARGET").unwrap();
    let mingw = target.contains("windows-gnu");
    cflags.push_str(" -ffunction-sections -fdata-sections");

    if target.contains("i686") {
        cflags.push_str(" -m32");
    } else if target.as_slice().contains("x86_64") {
        cflags.push_str(" -m64");
    }
    if !target.contains("i686") {
        cflags.push_str(" -fPIC");
    }

    let src = Path::new(os::getenv("CARGO_MANIFEST_DIR").unwrap());
    let dst = Path::new(os::getenv("OUT_DIR").unwrap());
    let _ = fs::mkdir(&dst.join("build"), io::USER_DIR);

    let mut cmd = Command::new("cmake");
    cmd.arg(src.join("libgit2"))
       .cwd(&dst.join("build"));
    if mingw {
        cmd.arg("-G").arg("Unix Makefiles");
    }
    let profile = match os::getenv("PROFILE").unwrap().as_slice() {
        "bench" | "release" => "Release",
        _ => "Debug",
    };
    run(cmd.arg("-DTHREADSAFE=ON")
           .arg("-DBUILD_SHARED_LIBS=OFF")
           .arg("-DBUILD_CLAR=OFF")
           .arg(format!("-DCMAKE_BUILD_TYPE={}", profile))
           .arg(format!("-DCMAKE_INSTALL_PREFIX={}", dst.display()))
           .arg("-DBUILD_EXAMPLES=OFF")
           .arg(format!("-DCMAKE_C_FLAGS={}", cflags)));
    run(Command::new("cmake")
                .arg("--build").arg(".")
                .arg("--target").arg("install")
                .cwd(&dst.join("build")));

    println!("cargo:root={}", dst.display());
    if mingw || target.contains("windows") {
        println!("cargo:rustc-flags=-l winhttp -l rpcrt4 -l ole32 \
                                    -l ws2_32 -l bcrypt -l crypt32 \
                                    -l git2:static -L {}",
                 dst.join("lib").display());
    } else if os::getenv("HOST") == os::getenv("TARGET") {
        opts.statik = true;
        opts.atleast_version = None;
        append("PKG_CONFIG_PATH", dst.join("lib/pkgconfig"));
        pkg_config::find_library_opts("libgit2", &opts).unwrap();
    } else {
        println!("cargo:rustc-flags=-l git2:static");
        println!("cargo:rustc-flags=-L {}", dst.join("lib").display());
        if target.contains("apple") {
            println!("cargo:rustc-flags:-l iconv");
        }
    }
}

fn run(cmd: &mut Command) {
    println!("running: {}", cmd);
    assert!(cmd.stdout(InheritFd(1))
               .stderr(InheritFd(2))
               .status()
               .unwrap()
               .success());

}

fn register_dep(dep: &str) {
    match os::getenv(format!("DEP_{}_ROOT", dep).as_slice()) {
        Some(s) => {
            append("CMAKE_PREFIX_PATH", Path::new(s.as_slice()));
            append("PKG_CONFIG_PATH", Path::new(s.as_slice()).join("lib/pkgconfig"));
        }
        None => {}
    }
}

fn append(var: &str, val: Path) {
    let prefix = os::getenv(var).unwrap_or(String::new());
    let mut v = os::split_paths(prefix.as_slice());
    v.push(val);
    os::setenv(var, os::join_paths(v.as_slice()).unwrap());
}
