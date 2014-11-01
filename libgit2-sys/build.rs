extern crate "pkg-config" as pkg_config;

use std::os;
use std::io::{mod, fs, Command};
use std::io::process::InheritFd;

fn main() {
    let mut opts = pkg_config::default_options("libgit2");
    opts.atleast_version = Some("0.21.0".to_string());
    match pkg_config::find_library_opts("libgit2", &opts) {
        Ok(()) => return,
        Err(..) => {}
    }

    match os::getenv("DEP_SSH2_ROOT") {
        Some(s) => {
            let prefix = os::getenv("CMAKE_PREFIX_PATH").unwrap_or(String::new());
            let mut v = os::split_paths(prefix.as_slice());
            v.push(Path::new(s));
            os::setenv("CMAKE_PREFIX_PATH", os::join_paths(v.as_slice()).unwrap());
        }
        None => {}
    }

    let mut cflags = os::getenv("CFLAGS").unwrap_or(String::new());
    let target = os::getenv("TARGET").unwrap();
    let mingw = target.contains("mingw");
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
    run(cmd.arg("-DTHREADSAFE=ON")
           .arg("-DBUILD_SHARED_LIBS=OFF")
           .arg("-DBUILD_CLAR=OFF")
           .arg("-DCMAKE_BUILD_TYPE=RelWithDebInfo")
           .arg(format!("-DCMAKE_INSTALL_PREFIX={}", dst.display()))
           .arg("-DBUILD_EXAMPLES=OFF")
           .arg(format!("-DCMAKE_C_FLAGS={}", cflags)));
    run(Command::new("cmake")
                .arg("--build").arg(".")
                .arg("--target").arg("install")
                .cwd(&dst.join("build")));

    println!("cargo:rustc-flags=-L {}", dst.join("lib").display());
    println!("cargo:root={}", dst.display());
    if mingw || target.contains("windows") {
        println!("cargo:rustc-flags=-l winhttp -l rpcrt4 -l ole32 \
                                    -l ws2_32 -l bcrypt -l crypt32 \
                                    -l git2:static");
    } else {
        println!("cargo:rustc-flags=-l git2:static");
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
