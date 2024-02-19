use libc::{c_char, c_int, size_t};
use std::cmp::Ordering;
use std::ffi::{CString, OsStr, OsString};
use std::path::{Component, Path, PathBuf};

use crate::{raw, Error};

#[doc(hidden)]
pub trait IsNull {
    fn is_ptr_null(&self) -> bool;
}
impl<T> IsNull for *const T {
    fn is_ptr_null(&self) -> bool {
        self.is_null()
    }
}
impl<T> IsNull for *mut T {
    fn is_ptr_null(&self) -> bool {
        self.is_null()
    }
}

#[doc(hidden)]
pub trait Binding: Sized {
    type Raw;

    unsafe fn from_raw(raw: Self::Raw) -> Self;
    fn raw(&self) -> Self::Raw;

    unsafe fn from_raw_opt<T>(raw: T) -> Option<Self>
    where
        T: Copy + IsNull,
        Self: Binding<Raw = T>,
    {
        if raw.is_ptr_null() {
            None
        } else {
            Some(Binding::from_raw(raw))
        }
    }
}

/// Converts an iterator of repo paths into a git2-compatible array of cstrings.
///
/// Only use this for repo-relative paths or pathspecs.
///
/// See `iter2cstrs` for more details.
pub fn iter2cstrs_paths<T, I>(
    iter: I,
) -> Result<(Vec<CString>, Vec<*const c_char>, raw::git_strarray), Error>
where
    T: IntoCString,
    I: IntoIterator<Item = T>,
{
    let cstrs = iter
        .into_iter()
        .map(|i| fixup_windows_path(i.into_c_string()?))
        .collect::<Result<Vec<CString>, _>>()?;
    iter2cstrs(cstrs)
}

/// Converts an iterator of things into a git array of c-strings.
///
/// Returns a tuple `(cstrings, pointers, git_strarray)`. The first two values
/// should not be dropped before `git_strarray`.
pub fn iter2cstrs<T, I>(
    iter: I,
) -> Result<(Vec<CString>, Vec<*const c_char>, raw::git_strarray), Error>
where
    T: IntoCString,
    I: IntoIterator<Item = T>,
{
    let cstrs = iter
        .into_iter()
        .map(|i| i.into_c_string())
        .collect::<Result<Vec<CString>, _>>()?;
    let ptrs = cstrs.iter().map(|i| i.as_ptr()).collect::<Vec<_>>();
    let raw = raw::git_strarray {
        strings: ptrs.as_ptr() as *mut _,
        count: ptrs.len() as size_t,
    };
    Ok((cstrs, ptrs, raw))
}

#[cfg(unix)]
pub fn bytes2path(b: &[u8]) -> &Path {
    use std::os::unix::prelude::*;
    Path::new(OsStr::from_bytes(b))
}
#[cfg(windows)]
pub fn bytes2path(b: &[u8]) -> &Path {
    use std::str;
    Path::new(str::from_utf8(b).unwrap())
}

/// A class of types that can be converted to C strings.
///
/// These types are represented internally as byte slices and it is quite rare
/// for them to contain an interior 0 byte.
pub trait IntoCString {
    /// Consume this container, converting it into a CString
    fn into_c_string(self) -> Result<CString, Error>;
}

impl<'a, T: IntoCString + Clone> IntoCString for &'a T {
    fn into_c_string(self) -> Result<CString, Error> {
        self.clone().into_c_string()
    }
}

impl<'a> IntoCString for &'a str {
    fn into_c_string(self) -> Result<CString, Error> {
        Ok(CString::new(self)?)
    }
}

impl IntoCString for String {
    fn into_c_string(self) -> Result<CString, Error> {
        Ok(CString::new(self.into_bytes())?)
    }
}

impl IntoCString for CString {
    fn into_c_string(self) -> Result<CString, Error> {
        Ok(self)
    }
}

impl<'a> IntoCString for &'a Path {
    fn into_c_string(self) -> Result<CString, Error> {
        let s: &OsStr = self.as_ref();
        s.into_c_string()
    }
}

impl IntoCString for PathBuf {
    fn into_c_string(self) -> Result<CString, Error> {
        let s: OsString = self.into();
        s.into_c_string()
    }
}

impl<'a> IntoCString for &'a OsStr {
    fn into_c_string(self) -> Result<CString, Error> {
        self.to_os_string().into_c_string()
    }
}

impl IntoCString for OsString {
    #[cfg(unix)]
    fn into_c_string(self) -> Result<CString, Error> {
        use std::os::unix::prelude::*;
        let s: &OsStr = self.as_ref();
        Ok(CString::new(s.as_bytes())?)
    }
    #[cfg(windows)]
    fn into_c_string(self) -> Result<CString, Error> {
        match self.to_str() {
            Some(s) => s.into_c_string(),
            None => Err(Error::from_str(
                "only valid unicode paths are accepted on windows",
            )),
        }
    }
}

impl<'a> IntoCString for &'a [u8] {
    fn into_c_string(self) -> Result<CString, Error> {
        Ok(CString::new(self)?)
    }
}

impl IntoCString for Vec<u8> {
    fn into_c_string(self) -> Result<CString, Error> {
        Ok(CString::new(self)?)
    }
}

pub fn into_opt_c_string<S>(opt_s: Option<S>) -> Result<Option<CString>, Error>
where
    S: IntoCString,
{
    match opt_s {
        None => Ok(None),
        Some(s) => Ok(Some(s.into_c_string()?)),
    }
}

pub fn c_cmp_to_ordering(cmp: c_int) -> Ordering {
    match cmp {
        0 => Ordering::Equal,
        n if n < 0 => Ordering::Less,
        _ => Ordering::Greater,
    }
}

/// Converts a path to a CString that is usable by the libgit2 API.
///
/// Checks if it is a relative path.
///
/// On Windows, this also requires the path to be valid Unicode, and translates
/// back slashes to forward slashes.
pub fn path_to_repo_path(path: &Path) -> Result<CString, Error> {
    macro_rules! err {
        ($msg:literal, $path:expr) => {
            return Err(Error::from_str(&format!($msg, $path.display())))
        };
    }
    match path.components().next() {
        None => return Err(Error::from_str("repo path should not be empty")),
        Some(Component::Prefix(_)) => err!(
            "repo path `{}` should be relative, not a windows prefix",
            path
        ),
        Some(Component::RootDir) => err!("repo path `{}` should be relative", path),
        Some(Component::CurDir) => err!("repo path `{}` should not start with `.`", path),
        Some(Component::ParentDir) => err!("repo path `{}` should not start with `..`", path),
        Some(Component::Normal(_)) => {}
    }
    #[cfg(windows)]
    {
        match path.to_str() {
            None => {
                return Err(Error::from_str(
                    "only valid unicode paths are accepted on windows",
                ))
            }
            Some(s) => return fixup_windows_path(s),
        }
    }
    #[cfg(not(windows))]
    {
        path.into_c_string()
    }
}

pub fn cstring_to_repo_path<T: IntoCString>(path: T) -> Result<CString, Error> {
    fixup_windows_path(path.into_c_string()?)
}

#[cfg(windows)]
fn fixup_windows_path<P: Into<Vec<u8>>>(path: P) -> Result<CString, Error> {
    let mut bytes: Vec<u8> = path.into();
    for i in 0..bytes.len() {
        if bytes[i] == b'\\' {
            bytes[i] = b'/';
        }
    }
    Ok(CString::new(bytes)?)
}

#[cfg(not(windows))]
fn fixup_windows_path(path: CString) -> Result<CString, Error> {
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_err {
        ($path:expr, $msg:expr) => {
            match path_to_repo_path(Path::new($path)) {
                Ok(_) => panic!("expected `{}` to err", $path),
                Err(e) => assert_eq!(e.message(), $msg),
            }
        };
    }

    macro_rules! assert_repo_path_ok {
        ($path:expr) => {
            assert_repo_path_ok!($path, $path)
        };
        ($path:expr, $expect:expr) => {
            assert_eq!(
                path_to_repo_path(Path::new($path)),
                Ok(CString::new($expect).unwrap())
            );
        };
    }

    #[test]
    #[cfg(windows)]
    fn path_to_repo_path_translate() {
        assert_repo_path_ok!("foo");
        assert_repo_path_ok!("foo/bar");
        assert_repo_path_ok!(r"foo\bar", "foo/bar");
        assert_repo_path_ok!(r"foo\bar\", "foo/bar/");
    }

    #[test]
    fn path_to_repo_path_no_weird() {
        assert_err!("", "repo path should not be empty");
        assert_err!("./foo", "repo path `./foo` should not start with `.`");
        assert_err!("../foo", "repo path `../foo` should not start with `..`");
    }

    #[test]
    #[cfg(not(windows))]
    fn path_to_repo_path_no_absolute() {
        assert_err!("/", "repo path `/` should be relative");
        assert_repo_path_ok!("foo/bar");
    }

    #[test]
    #[cfg(windows)]
    fn path_to_repo_path_no_absolute() {
        assert_err!(
            r"c:",
            r"repo path `c:` should be relative, not a windows prefix"
        );
        assert_err!(
            r"c:\",
            r"repo path `c:\` should be relative, not a windows prefix"
        );
        assert_err!(
            r"c:temp",
            r"repo path `c:temp` should be relative, not a windows prefix"
        );
        assert_err!(
            r"\\?\UNC\a\b\c",
            r"repo path `\\?\UNC\a\b\c` should be relative, not a windows prefix"
        );
        assert_err!(
            r"\\?\c:\foo",
            r"repo path `\\?\c:\foo` should be relative, not a windows prefix"
        );
        assert_err!(
            r"\\.\COM42",
            r"repo path `\\.\COM42` should be relative, not a windows prefix"
        );
        assert_err!(
            r"\\a\b",
            r"repo path `\\a\b` should be relative, not a windows prefix"
        );
        assert_err!(r"\", r"repo path `\` should be relative");
        assert_err!(r"/", r"repo path `/` should be relative");
        assert_err!(r"\foo", r"repo path `\foo` should be relative");
        assert_err!(r"/foo", r"repo path `/foo` should be relative");
    }
}
