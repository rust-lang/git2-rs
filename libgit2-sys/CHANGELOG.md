# Changelog

## 0.14.2+1.5.1 - 2023-01-20
[0.14.1...0.14.2](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.14.1+1.5.0...libgit2-sys-0.14.2+1.5.1)

### Changed
- Updated the bundled libgit2 to [1.5.1](https://github.com/libgit2/libgit2/releases/tag/v1.5.1).
  [a233483a3952d6112653be86fb5ce65267e3d5ac](https://github.com/rust-lang/git2-rs/commit/a233483a3952d6112653be86fb5ce65267e3d5ac)
  - Changes: [fbea439d4b6fc91c6b619d01b85ab3b7746e4c19...42e5db98b963ae503229c63e44e06e439df50e56](https://github.com/libgit2/libgit2/compare/fbea439d4b6fc91c6b619d01b85ab3b7746e4c19...42e5db98b963ae503229c63e44e06e439df50e56):
  - Fixes [GHSA-8643-3wh5-rmjq](https://github.com/libgit2/libgit2/security/advisories/GHSA-8643-3wh5-rmjq) to validate SSH host keys.
  - The supported libgit2 system library range is 1.5.1 to less than 1.6.0 or 1.4.5 to less than 1.5.0, which should include this fix.

## 0.13.5+1.4.5 - 2023-01-20
[0.13.4...0.13.5](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.13.4+1.4.2...libgit2-sys-0.13.5+1.4.5)

### Changed
- Updated the bundled libgit2 to [1.4.5](https://github.com/libgit2/libgit2/releases/tag/v1.4.5).
  - Changes: [2a0d0bd19b5d13e2ab7f3780e094404828cbb9a7...cd6f679af401eda1f172402006ef8265f8bd58ea](https://github.com/libgit2/libgit2/compare/2a0d0bd19b5d13e2ab7f3780e094404828cbb9a7...cd6f679af401eda1f172402006ef8265f8bd58ea):
  - Fixes [GHSA-8643-3wh5-rmjq](https://github.com/libgit2/libgit2/security/advisories/GHSA-8643-3wh5-rmjq) to validate SSH host keys.
  - The supported libgit2 system library range is 1.4.5 to less than 1.5.0.

## 0.14.1+1.5.0 - 2023-01-10
[0.14.0...0.14.1](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.14.0+1.5.0...libgit2-sys-0.14.1+1.5.0)

### Added
- Added variants to `git_cert_ssh_raw_type_t`.
  [#909](https://github.com/rust-lang/git2-rs/pull/909)

## 0.14.0+1.5.0 - 2022-07-28
[0.13.4...0.14.0](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.13.4+1.4.2...libgit2-sys-0.14.0+1.5.0)

### Added
- Added bindings for ownership validation.
  [#839](https://github.com/rust-lang/git2-rs/pull/839)

### Changed

- Updated the bundled libgit2 to [1.5.0](https://github.com/libgit2/libgit2/releases/tag/v1.5.0).
  [#839](https://github.com/rust-lang/git2-rs/pull/839)
  [#858](https://github.com/rust-lang/git2-rs/pull/858)
  - Changes: [2a0d0bd19b5d13e2ab7f3780e094404828cbb9a7...fbea439d4b6fc91c6b619d01b85ab3b7746e4c19](https://github.com/libgit2/libgit2/compare/2a0d0bd19b5d13e2ab7f3780e094404828cbb9a7...fbea439d4b6fc91c6b619d01b85ab3b7746e4c19):
  - The supported libgit2 system library range is 1.4.4 to less than 1.6.0.
  - Fixes [CVE 2022-24765](https://github.com/libgit2/libgit2/releases/tag/v1.4.3).

## 0.13.4+1.4.2 - 2022-05-10
[0.13.3...0.13.4](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.13.3+1.4.2...libgit2-sys-0.13.4+1.4.2)

### Added
- Added bindings for `git_commit_body`
  [#835](https://github.com/rust-lang/git2-rs/pull/835)

## 0.13.3+1.4.2 - 2022-04-27
[0.13.2...0.13.3](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.13.2+1.4.2...libgit2-sys-0.13.3+1.4.2)

### Changed
- Updated the bundled libgit2 to 1.5.0-alpha.
  [#822](https://github.com/rust-lang/git2-rs/pull/822)
  - Changes: [182d0d1ee933de46bf0b5a6ec269bafa77aba9a2...2a0d0bd19b5d13e2ab7f3780e094404828cbb9a7](https://github.com/libgit2/libgit2/compare/182d0d1ee933de46bf0b5a6ec269bafa77aba9a2...2a0d0bd19b5d13e2ab7f3780e094404828cbb9a7)
- Changed the pkg-config probe to restrict linking against a version of a system-installed libgit2 to a version less than 1.5.0.
  Previously it would allow any version above 1.4.0 which could pick up an API-breaking version.
  [#817](https://github.com/rust-lang/git2-rs/pull/817)
- When using pkg-config to locate libgit2, the system lib dirs are no longer added to the search path.
  [#831](https://github.com/rust-lang/git2-rs/pull/831)
- When using the `zlib-ng-compat` Cargo feature, `libssh2-sys` is no longer automatically included unless you also enable the `ssh` feature.
  [#833](https://github.com/rust-lang/git2-rs/pull/833)

## 0.13.2+1.4.2 - 2022-03-10
[0.13.1...0.13.2](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.13.1+1.4.2...libgit2-sys-0.13.2+1.4.2)

### Added
- Added bindings for `git_odb_exists_ext`.
  [#818](https://github.com/rust-lang/git2-rs/pull/818)

## 0.13.1+1.4.2 - 2022-02-28
[0.13.0...0.13.1](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.13.0+1.4.1...libgit2-sys-0.13.1+1.4.2)

### Changed
- Updated the bundled libgit2 to [1.4.2](https://github.com/libgit2/libgit2/releases/tag/v1.4.2).
  [#815](https://github.com/rust-lang/git2-rs/pull/815)
  - Changes: [fdd15bcfca6b2ec4b7ecad1aa11a396cb15bd064...182d0d1ee933de46bf0b5a6ec269bafa77aba9a2](https://github.com/libgit2/libgit2/compare/fdd15bcfca6b2ec4b7ecad1aa11a396cb15bd064...182d0d1ee933de46bf0b5a6ec269bafa77aba9a2).

## 0.13.0+1.4.1 - 2022-02-24
[0.12.26...0.13.0](https://github.com/rust-lang/git2-rs/compare/libgit2-sys-0.12.26+1.3.0...libgit2-sys-0.13.0+1.4.1)

### Changed
- Changed libgit2-sys to use the presence of the `src` directory instead of `.git` to determine if it has a git submodule that needs updating.
  [#801](https://github.com/rust-lang/git2-rs/pull/801)
- Updated the bundled libgit2 to [1.4.1](https://github.com/libgit2/libgit2/releases/tag/v1.4.1) (see also [1.4.0](https://github.com/libgit2/libgit2/releases/tag/v1.4.0))
  [#806](https://github.com/rust-lang/git2-rs/pull/806)
  [#811](https://github.com/rust-lang/git2-rs/pull/811)
  - Changes: [b7bad55e4bb0a285b073ba5e02b01d3f522fc95d...fdd15bcfca6b2ec4b7ecad1aa11a396cb15bd064](https://github.com/libgit2/libgit2/compare/b7bad55e4bb0a285b073ba5e02b01d3f522fc95d...fdd15bcfca6b2ec4b7ecad1aa11a396cb15bd064)
  - The supported libgit2 system library range is 1.4.0 or greater.
