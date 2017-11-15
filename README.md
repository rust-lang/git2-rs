# git2-rs

[![Build Status](https://travis-ci.org/alexcrichton/git2-rs.svg?branch=master)](https://travis-ci.org/alexcrichton/git2-rs)
[![Build Status](https://ci.appveyor.com/api/projects/status/6vem3xgno2kuxnfm?svg=true)](https://ci.appveyor.com/project/alexcrichton/git2-rs)

[Documentation](https://docs.rs/git2)

libgit2 bindings for Rust

```toml
[dependencies]
git2 = "0.6"
```

## Version of libgit2

Currently this library requires libgit2 0.25.1. The source for libgit2 is
included in the libgit2-sys crate so there's no need to pre-install the libgit2
library, the libgit2-sys crate will figure that and/or build that for you.

## Building git2-rs

First, you'll need to install _CMake_. Afterwards, just run:

```sh
$ git clone https://github.com/alexcrichton/git2-rs
$ cd git2-rs
$ cargo build
```

## Building on OSX 10.10+

Currently libssh2 requires linking against OpenSSL, and to compile libssh2 it
also needs to find the OpenSSL headers. On OSX 10.10+ the OpenSSL headers have
been removed, but if you're using Homebrew you can install them via:

```sh
brew install openssl
```


# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in git2-rs by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
