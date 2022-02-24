# Contributing

## Updating libgit2

The following steps can be used to update libgit2:

1. Update the submodule.
   There are several ways to go about this.
   One way is to go to the `libgit2-sys/libgit2` directory and run `git fetch origin` to download the latest updates, and then check out a specific tag (such as `git checkout v1.4.1`).
2. Update all the references to the version:
    * Update [`libgit2-sys/build.rs`](https://github.com/rust-lang/git2-rs/blob/master/libgit2-sys/build.rs).
      There is a version probe (search for `cfg.atleast_version`) which should be updated.
    * Update the version in
      [`libgit2-sys/Cargo.toml`](https://github.com/rust-lang/git2-rs/blob/master/libgit2-sys/Cargo.toml).
      Update the metadata portion (the part after the `+`) to match libgit2.
      Also bump the Cargo version (the part before the `+`), keeping in mind
      if this will be a SemVer breaking change or not.
    * Update the dependency version in [`Cargo.toml`](https://github.com/rust-lang/git2-rs/blob/master/Cargo.toml) to match the version in the last step (do not include the `+` metadata).
      Also update the version of the `git2` crate itself so it will pick up the change to `libgit2-sys` (also keeping in mind if it is a SemVer breaking release).
    * Update the version in [`README.md`](https://github.com/rust-lang/git2-rs/blob/master/README.md) if needed.
    * If there was a SemVer-breaking version bump for either library, also update the `html_root_url` attribute in the `lib.rs` of each library.
3. Run tests.
   `cargo test -p git2 -p git2-curl` is a good starting point.
4. Run `systest`.
   This will validate for any C-level API problems.
   Unfortunately `systest` does not work on nightly, so you'll need to use stable.

   `cargo +stable run -p systest`

   The changelog at <https://github.com/libgit2/libgit2/blob/main/docs/changelog.md>
   can be helpful for seeing what has changed.
   The project has recently started labeling API and ABI breaking changes with labels:
   <https://github.com/libgit2/libgit2/pulls?q=is%3Apr+label%3A%22api+breaking%22%2C%22abi+breaking%22+is%3Aclosed>
4. Once you have everything functional, publish a PR with the updates.
