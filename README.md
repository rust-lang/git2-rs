# git2-rs

[Documentation](https://docs.rs/git2)

libgit2 bindings for Rust.

```toml
[dependencies]
git2 = "0.20.2"
```

## Rust version requirements

git2-rs works with stable Rust, and typically works with the most recent prior
stable release as well.

## Version of libgit2

Currently this library requires libgit2 1.9.0 (or newer patch versions). The
source for libgit2 is included in the libgit2-sys crate so there's no need to
pre-install the libgit2 library, the libgit2-sys crate will figure that and/or
build that for you. On the other hand, if an appropriate version of `libgit2`
is present, `git2` will attempt to dynamically link it.

To be more precise, the vendored `libgit2` is linked statically if two
conditions both hold:

- The environment variable `LIBGIT2_NO_VENDOR=1` is **not** set
- **and** either a) The Cargo feature `vendored-libgit2` is set or b) an
  appropriate version of `libgit2` cannot be found on the system.

In particular, note that the environment variable overrides the Cargo feature.

## Building git2-rs

```sh
$ git clone https://github.com/rust-lang/git2-rs
$ cd git2-rs
$ cargo build
```

### Automating Testing

Running tests and handling all of the associated edge cases on every commit
proves tedious very quickly.  To automate tests and handle proper stashing and
unstashing of unstaged changes and thus avoid nasty surprises, use the
pre-commit hook found [here][pre-commit-hook] and place it into the
`.git/hooks/` with the name `pre-commit`.  You may need to add execution
permissions with `chmod +x`.

To skip tests on a simple commit or doc-fixes, use `git commit --no-verify`.

## Building on macOS 10.10+

If the `ssh` feature is enabled (and it is by default) then this library depends
on libssh2 which depends on OpenSSL. To get OpenSSL working follow the
[`openssl` crate's instructions](https://github.com/sfackler/rust-openssl/blob/master/openssl/src/lib.rs#L31).

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in git2-rs by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[pre-commit-hook]: https://gist.github.com/glfmn/0c5e9e2b41b48007ed3497d11e3dbbfa
