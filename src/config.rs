use std::ffi::CString;
use std::marker;
use std::path::{Path, PathBuf};
use std::ptr;
use std::str;

use crate::util::{self, Binding};
use crate::{raw, Buf, ConfigLevel, Error, IntoCString};

/// A structure representing a git configuration key/value store
pub struct Config {
    raw: *mut raw::git_config,
}

/// A struct representing a certain entry owned by a `Config` instance.
///
/// An entry has a name, a value, and a level it applies to.
pub struct ConfigEntry<'cfg> {
    raw: *mut raw::git_config_entry,
    _marker: marker::PhantomData<&'cfg Config>,
    owned: bool,
}

/// An iterator over the `ConfigEntry` values of a `Config` structure.
///
/// Due to lifetime restrictions, `ConfigEntries` does not implement the
/// standard [`Iterator`] trait. It provides a [`next`] function which only
/// allows access to one entry at a time. [`for_each`] is available as a
/// convenience function.
///
/// [`next`]: ConfigEntries::next
/// [`for_each`]: ConfigEntries::for_each
///
/// # Example
///
/// ```
/// // Example of how to collect all entries.
/// use git2::Config;
///
/// let config = Config::new()?;
/// let iter = config.entries(None)?;
/// let mut entries = Vec::new();
/// iter
///     .for_each(|entry| {
///         let name = entry.name().unwrap().to_string();
///         let value = entry.value().unwrap_or("").to_string();
///         entries.push((name, value))
///     })?;
/// for entry in &entries {
///     println!("{} = {}", entry.0, entry.1);
/// }
/// # Ok::<(), git2::Error>(())
///
/// ```
pub struct ConfigEntries<'cfg> {
    raw: *mut raw::git_config_iterator,
    current: Option<ConfigEntry<'cfg>>,
    _marker: marker::PhantomData<&'cfg Config>,
}

impl Config {
    /// Allocate a new configuration object
    ///
    /// This object is empty, so you have to add a file to it before you can do
    /// anything with it.
    pub fn new() -> Result<Config, Error> {
        crate::init();
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_config_new(&mut raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new config instance containing a single on-disk file
    pub fn open(path: &Path) -> Result<Config, Error> {
        crate::init();
        let mut raw = ptr::null_mut();
        // Normal file path OK (does not need Windows conversion).
        let path = path.into_c_string()?;
        unsafe {
            try_call!(raw::git_config_open_ondisk(&mut raw, path));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Open the global, XDG and system configuration files
    ///
    /// Utility wrapper that finds the global, XDG and system configuration
    /// files and opens them into a single prioritized config object that can
    /// be used when accessing default config data outside a repository.
    pub fn open_default() -> Result<Config, Error> {
        crate::init();
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_config_open_default(&mut raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Locate the path to the global configuration file
    ///
    /// The user or global configuration file is usually located in
    /// `$HOME/.gitconfig`.
    ///
    /// This method will try to guess the full path to that file, if the file
    /// exists. The returned path may be used on any method call to load
    /// the global configuration file.
    ///
    /// This method will not guess the path to the XDG compatible config file
    /// (`.config/git/config`).
    pub fn find_global() -> Result<PathBuf, Error> {
        crate::init();
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_config_find_global(buf.raw()));
        }
        Ok(util::bytes2path(&buf).to_path_buf())
    }

    /// Locate the path to the system configuration file
    ///
    /// If /etc/gitconfig doesn't exist, it will look for `%PROGRAMFILES%`
    pub fn find_system() -> Result<PathBuf, Error> {
        crate::init();
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_config_find_system(buf.raw()));
        }
        Ok(util::bytes2path(&buf).to_path_buf())
    }

    /// Locate the path to the global XDG compatible configuration file
    ///
    /// The XDG compatible configuration file is usually located in
    /// `$HOME/.config/git/config`.
    pub fn find_xdg() -> Result<PathBuf, Error> {
        crate::init();
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_config_find_xdg(buf.raw()));
        }
        Ok(util::bytes2path(&buf).to_path_buf())
    }

    /// Add an on-disk config file instance to an existing config
    ///
    /// The on-disk file pointed at by path will be opened and parsed; it's
    /// expected to be a native Git config file following the default Git config
    /// syntax (see man git-config).
    ///
    /// Further queries on this config object will access each of the config
    /// file instances in order (instances with a higher priority level will be
    /// accessed first).
    pub fn add_file(&mut self, path: &Path, level: ConfigLevel, force: bool) -> Result<(), Error> {
        // Normal file path OK (does not need Windows conversion).
        let path = path.into_c_string()?;
        unsafe {
            try_call!(raw::git_config_add_file_ondisk(
                self.raw,
                path,
                level,
                ptr::null(),
                force
            ));
            Ok(())
        }
    }

    /// Delete a config variable from the config file with the highest level
    /// (usually the local one).
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_delete_entry(self.raw, name));
            Ok(())
        }
    }

    /// Remove multivar config variables in the config file with the highest level (usually the
    /// local one).
    ///
    /// The regular expression is applied case-sensitively on the value.
    pub fn remove_multivar(&mut self, name: &str, regexp: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let regexp = CString::new(regexp)?;
        unsafe {
            try_call!(raw::git_config_delete_multivar(self.raw, name, regexp));
        }
        Ok(())
    }

    /// Get the value of a boolean config variable.
    ///
    /// All config files will be looked into, in the order of their defined
    /// level. A higher level means a higher priority. The first occurrence of
    /// the variable will be returned here.
    pub fn get_bool(&self, name: &str) -> Result<bool, Error> {
        let mut out = 0 as libc::c_int;
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_bool(&mut out, &*self.raw, name));
        }
        Ok(out != 0)
    }

    /// Get the value of an integer config variable.
    ///
    /// All config files will be looked into, in the order of their defined
    /// level. A higher level means a higher priority. The first occurrence of
    /// the variable will be returned here.
    pub fn get_i32(&self, name: &str) -> Result<i32, Error> {
        let mut out = 0i32;
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_int32(&mut out, &*self.raw, name));
        }
        Ok(out)
    }

    /// Get the value of an integer config variable.
    ///
    /// All config files will be looked into, in the order of their defined
    /// level. A higher level means a higher priority. The first occurrence of
    /// the variable will be returned here.
    pub fn get_i64(&self, name: &str) -> Result<i64, Error> {
        let mut out = 0i64;
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_int64(&mut out, &*self.raw, name));
        }
        Ok(out)
    }

    /// Get the value of a string config variable.
    ///
    /// This is the same as `get_bytes` except that it may return `Err` if
    /// the bytes are not valid utf-8.
    ///
    /// This method will return an error if this `Config` is not a snapshot.
    pub fn get_str(&self, name: &str) -> Result<&str, Error> {
        str::from_utf8(self.get_bytes(name)?)
            .map_err(|_| Error::from_str("configuration value is not valid utf8"))
    }

    /// Get the value of a string config variable as a byte slice.
    ///
    /// This method will return an error if this `Config` is not a snapshot.
    pub fn get_bytes(&self, name: &str) -> Result<&[u8], Error> {
        let mut ret = ptr::null();
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_string(&mut ret, &*self.raw, name));
            Ok(crate::opt_bytes(self, ret).unwrap())
        }
    }

    /// Get the value of a string config variable as an owned string.
    ///
    /// All config files will be looked into, in the order of their
    /// defined level. A higher level means a higher priority. The
    /// first occurrence of the variable will be returned here.
    ///
    /// An error will be returned if the config value is not valid utf-8.
    pub fn get_string(&self, name: &str) -> Result<String, Error> {
        let ret = Buf::new();
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_string_buf(ret.raw(), self.raw, name));
        }
        str::from_utf8(&ret)
            .map(|s| s.to_string())
            .map_err(|_| Error::from_str("configuration value is not valid utf8"))
    }

    /// Get the value of a path config variable as an owned `PathBuf`.
    ///
    /// A leading '~' will be expanded to the global search path (which
    /// defaults to the user's home directory but can be overridden via
    /// [`raw::git_libgit2_opts`].
    ///
    /// All config files will be looked into, in the order of their
    /// defined level. A higher level means a higher priority. The
    /// first occurrence of the variable will be returned here.
    pub fn get_path(&self, name: &str) -> Result<PathBuf, Error> {
        let ret = Buf::new();
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_path(ret.raw(), self.raw, name));
        }
        Ok(crate::util::bytes2path(&ret).to_path_buf())
    }

    /// Get the ConfigEntry for a config variable.
    pub fn get_entry(&self, name: &str) -> Result<ConfigEntry<'_>, Error> {
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_get_entry(&mut ret, self.raw, name));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Iterate over all the config variables
    ///
    /// If `glob` is `Some`, then the iterator will only iterate over all
    /// variables whose name matches the pattern.
    ///
    /// The regular expression is applied case-sensitively on the normalized form of
    /// the variable name: the section and variable parts are lower-cased. The
    /// subsection is left unchanged.
    ///
    /// Due to lifetime restrictions, the returned value does not implement
    /// the standard [`Iterator`] trait. See [`ConfigEntries`] for more.
    ///
    /// # Example
    ///
    /// ```
    /// use git2::Config;
    ///
    /// let cfg = Config::new().unwrap();
    ///
    /// let mut entries = cfg.entries(None).unwrap();
    /// while let Some(entry) = entries.next() {
    ///     let entry = entry.unwrap();
    ///     println!("{} => {}", entry.name().unwrap(), entry.value().unwrap());
    /// }
    /// ```
    pub fn entries(&self, glob: Option<&str>) -> Result<ConfigEntries<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            match glob {
                Some(s) => {
                    let s = CString::new(s)?;
                    try_call!(raw::git_config_iterator_glob_new(&mut ret, &*self.raw, s));
                }
                None => {
                    try_call!(raw::git_config_iterator_new(&mut ret, &*self.raw));
                }
            }
            Ok(Binding::from_raw(ret))
        }
    }

    /// Iterate over the values of a multivar
    ///
    /// If `regexp` is `Some`, then the iterator will only iterate over all
    /// values which match the pattern.
    ///
    /// The regular expression is applied case-sensitively on the normalized form of
    /// the variable name: the section and variable parts are lower-cased. The
    /// subsection is left unchanged.
    ///
    /// Due to lifetime restrictions, the returned value does not implement
    /// the standard [`Iterator`] trait. See [`ConfigEntries`] for more.
    pub fn multivar(&self, name: &str, regexp: Option<&str>) -> Result<ConfigEntries<'_>, Error> {
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        let regexp = regexp.map(CString::new).transpose()?;
        unsafe {
            try_call!(raw::git_config_multivar_iterator_new(
                &mut ret, &*self.raw, name, regexp
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Open the global/XDG configuration file according to git's rules
    ///
    /// Git allows you to store your global configuration at `$HOME/.config` or
    /// `$XDG_CONFIG_HOME/git/config`. For backwards compatibility, the XDG file
    /// shouldn't be used unless the use has created it explicitly. With this
    /// function you'll open the correct one to write to.
    pub fn open_global(&mut self) -> Result<Config, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_config_open_global(&mut raw, self.raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Build a single-level focused config object from a multi-level one.
    ///
    /// The returned config object can be used to perform get/set/delete
    /// operations on a single specific level.
    pub fn open_level(&self, level: ConfigLevel) -> Result<Config, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_config_open_level(&mut raw, &*self.raw, level));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Set the value of a boolean config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_bool(&mut self, name: &str, value: bool) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_set_bool(self.raw, name, value));
        }
        Ok(())
    }

    /// Set the value of an integer config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_i32(&mut self, name: &str, value: i32) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_set_int32(self.raw, name, value));
        }
        Ok(())
    }

    /// Set the value of an integer config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_i64(&mut self, name: &str, value: i64) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_config_set_int64(self.raw, name, value));
        }
        Ok(())
    }

    /// Set the value of an multivar config variable in the config file with the
    /// highest level (usually the local one).
    ///
    /// The regular expression is applied case-sensitively on the value.
    pub fn set_multivar(&mut self, name: &str, regexp: &str, value: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let regexp = CString::new(regexp)?;
        let value = CString::new(value)?;
        unsafe {
            try_call!(raw::git_config_set_multivar(self.raw, name, regexp, value));
        }
        Ok(())
    }

    /// Set the value of a string config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_str(&mut self, name: &str, value: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let value = CString::new(value)?;
        unsafe {
            try_call!(raw::git_config_set_string(self.raw, name, value));
        }
        Ok(())
    }

    /// Create a snapshot of the configuration
    ///
    /// Create a snapshot of the current state of a configuration, which allows
    /// you to look into a consistent view of the configuration for looking up
    /// complex values (e.g. a remote, submodule).
    pub fn snapshot(&mut self) -> Result<Config, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_config_snapshot(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Parse a string as a bool.
    ///
    /// Interprets "true", "yes", "on", 1, or any non-zero number as true.
    /// Interprets "false", "no", "off", 0, or an empty string as false.
    pub fn parse_bool<S: IntoCString>(s: S) -> Result<bool, Error> {
        let s = s.into_c_string()?;
        let mut out = 0;
        crate::init();
        unsafe {
            try_call!(raw::git_config_parse_bool(&mut out, s));
        }
        Ok(out != 0)
    }

    /// Parse a string as an i32; handles suffixes like k, M, or G, and
    /// multiplies by the appropriate power of 1024.
    pub fn parse_i32<S: IntoCString>(s: S) -> Result<i32, Error> {
        let s = s.into_c_string()?;
        let mut out = 0;
        crate::init();
        unsafe {
            try_call!(raw::git_config_parse_int32(&mut out, s));
        }
        Ok(out)
    }

    /// Parse a string as an i64; handles suffixes like k, M, or G, and
    /// multiplies by the appropriate power of 1024.
    pub fn parse_i64<S: IntoCString>(s: S) -> Result<i64, Error> {
        let s = s.into_c_string()?;
        let mut out = 0;
        crate::init();
        unsafe {
            try_call!(raw::git_config_parse_int64(&mut out, s));
        }
        Ok(out)
    }
}

impl Binding for Config {
    type Raw = *mut raw::git_config;
    unsafe fn from_raw(raw: *mut raw::git_config) -> Config {
        Config { raw }
    }
    fn raw(&self) -> *mut raw::git_config {
        self.raw
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        unsafe { raw::git_config_free(self.raw) }
    }
}

impl<'cfg> ConfigEntry<'cfg> {
    /// Gets the name of this entry.
    ///
    /// May return `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes()).ok()
    }

    /// Gets the name of this entry as a byte slice.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, (*self.raw).name).unwrap() }
    }

    /// Gets the value of this entry.
    ///
    /// May return `None` if the value is not valid utf-8
    ///
    /// # Panics
    ///
    /// Panics when no value is defined.
    pub fn value(&self) -> Option<&str> {
        str::from_utf8(self.value_bytes()).ok()
    }

    /// Gets the value of this entry as a byte slice.
    ///
    /// # Panics
    ///
    /// Panics when no value is defined.
    pub fn value_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, (*self.raw).value).unwrap() }
    }

    /// Returns `true` when a value is defined otherwise `false`.
    ///
    /// No value defined is a short-hand to represent a Boolean `true`.
    pub fn has_value(&self) -> bool {
        unsafe { !(*self.raw).value.is_null() }
    }

    /// Gets the configuration level of this entry.
    pub fn level(&self) -> ConfigLevel {
        unsafe { ConfigLevel::from_raw((*self.raw).level) }
    }

    /// Depth of includes where this variable was found
    pub fn include_depth(&self) -> u32 {
        unsafe { (*self.raw).include_depth as u32 }
    }
}

impl<'cfg> Binding for ConfigEntry<'cfg> {
    type Raw = *mut raw::git_config_entry;

    unsafe fn from_raw(raw: *mut raw::git_config_entry) -> ConfigEntry<'cfg> {
        ConfigEntry {
            raw,
            _marker: marker::PhantomData,
            owned: true,
        }
    }
    fn raw(&self) -> *mut raw::git_config_entry {
        self.raw
    }
}

impl<'cfg> Binding for ConfigEntries<'cfg> {
    type Raw = *mut raw::git_config_iterator;

    unsafe fn from_raw(raw: *mut raw::git_config_iterator) -> ConfigEntries<'cfg> {
        ConfigEntries {
            raw,
            current: None,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_config_iterator {
        self.raw
    }
}

impl<'cfg> ConfigEntries<'cfg> {
    /// Advances the iterator and returns the next value.
    ///
    /// Returns `None` when iteration is finished.
    pub fn next(&mut self) -> Option<Result<&ConfigEntry<'cfg>, Error>> {
        let mut raw = ptr::null_mut();
        drop(self.current.take());
        unsafe {
            try_call_iter!(raw::git_config_next(&mut raw, self.raw));
            let entry = ConfigEntry {
                owned: false,
                raw,
                _marker: marker::PhantomData,
            };
            self.current = Some(entry);
            Some(Ok(self.current.as_ref().unwrap()))
        }
    }

    /// Calls the given closure for each remaining entry in the iterator.
    pub fn for_each<F: FnMut(&ConfigEntry<'cfg>)>(mut self, mut f: F) -> Result<(), Error> {
        while let Some(entry) = self.next() {
            let entry = entry?;
            f(entry);
        }
        Ok(())
    }
}

impl<'cfg> Drop for ConfigEntries<'cfg> {
    fn drop(&mut self) {
        unsafe { raw::git_config_iterator_free(self.raw) }
    }
}

impl<'cfg> Drop for ConfigEntry<'cfg> {
    fn drop(&mut self) {
        if self.owned {
            unsafe { raw::git_config_entry_free(self.raw) }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use tempfile::TempDir;

    use crate::Config;

    #[test]
    fn smoke() {
        let _cfg = Config::new().unwrap();
        let _ = Config::find_global();
        let _ = Config::find_system();
        let _ = Config::find_xdg();
    }

    #[test]
    fn persisted() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("foo");
        File::create(&path).unwrap();

        let mut cfg = Config::open(&path).unwrap();
        assert!(cfg.get_bool("foo.bar").is_err());
        cfg.set_bool("foo.k1", true).unwrap();
        cfg.set_i32("foo.k2", 1).unwrap();
        cfg.set_i64("foo.k3", 2).unwrap();
        cfg.set_str("foo.k4", "bar").unwrap();
        cfg.snapshot().unwrap();
        drop(cfg);

        let cfg = Config::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(cfg.get_bool("foo.k1").unwrap(), true);
        assert_eq!(cfg.get_i32("foo.k2").unwrap(), 1);
        assert_eq!(cfg.get_i64("foo.k3").unwrap(), 2);
        assert_eq!(cfg.get_str("foo.k4").unwrap(), "bar");

        let mut entries = cfg.entries(None).unwrap();
        while let Some(entry) = entries.next() {
            let entry = entry.unwrap();
            entry.name();
            entry.value();
            entry.level();
        }
    }

    #[test]
    fn multivar() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("foo");
        File::create(&path).unwrap();

        let mut cfg = Config::open(&path).unwrap();
        cfg.set_multivar("foo.bar", "^$", "baz").unwrap();
        cfg.set_multivar("foo.bar", "^$", "qux").unwrap();
        cfg.set_multivar("foo.bar", "^$", "quux").unwrap();
        cfg.set_multivar("foo.baz", "^$", "oki").unwrap();

        // `entries` filters by name
        let mut entries: Vec<String> = Vec::new();
        cfg.entries(Some("foo.bar"))
            .unwrap()
            .for_each(|entry| entries.push(entry.value().unwrap().to_string()))
            .unwrap();
        entries.sort();
        assert_eq!(entries, ["baz", "quux", "qux"]);

        // which is the same as `multivar` without a regex
        let mut multivals = Vec::new();
        cfg.multivar("foo.bar", None)
            .unwrap()
            .for_each(|entry| multivals.push(entry.value().unwrap().to_string()))
            .unwrap();
        multivals.sort();
        assert_eq!(multivals, entries);

        // yet _with_ a regex, `multivar` filters by value
        let mut quxish = Vec::new();
        cfg.multivar("foo.bar", Some("qu.*x"))
            .unwrap()
            .for_each(|entry| quxish.push(entry.value().unwrap().to_string()))
            .unwrap();
        quxish.sort();
        assert_eq!(quxish, ["quux", "qux"]);

        cfg.remove_multivar("foo.bar", ".*").unwrap();

        let count = |entries: super::ConfigEntries<'_>| -> usize {
            let mut c = 0;
            entries.for_each(|_| c += 1).unwrap();
            c
        };

        assert_eq!(count(cfg.entries(Some("foo.bar")).unwrap()), 0);
        assert_eq!(count(cfg.multivar("foo.bar", None).unwrap()), 0);
    }

    #[test]
    fn parse() {
        assert_eq!(Config::parse_bool("").unwrap(), false);
        assert_eq!(Config::parse_bool("false").unwrap(), false);
        assert_eq!(Config::parse_bool("no").unwrap(), false);
        assert_eq!(Config::parse_bool("off").unwrap(), false);
        assert_eq!(Config::parse_bool("0").unwrap(), false);

        assert_eq!(Config::parse_bool("true").unwrap(), true);
        assert_eq!(Config::parse_bool("yes").unwrap(), true);
        assert_eq!(Config::parse_bool("on").unwrap(), true);
        assert_eq!(Config::parse_bool("1").unwrap(), true);
        assert_eq!(Config::parse_bool("42").unwrap(), true);

        assert!(Config::parse_bool(" ").is_err());
        assert!(Config::parse_bool("some-string").is_err());
        assert!(Config::parse_bool("-").is_err());

        assert_eq!(Config::parse_i32("0").unwrap(), 0);
        assert_eq!(Config::parse_i32("1").unwrap(), 1);
        assert_eq!(Config::parse_i32("100").unwrap(), 100);
        assert_eq!(Config::parse_i32("-1").unwrap(), -1);
        assert_eq!(Config::parse_i32("-100").unwrap(), -100);
        assert_eq!(Config::parse_i32("1k").unwrap(), 1024);
        assert_eq!(Config::parse_i32("4k").unwrap(), 4096);
        assert_eq!(Config::parse_i32("1M").unwrap(), 1048576);
        assert_eq!(Config::parse_i32("1G").unwrap(), 1024 * 1024 * 1024);

        assert_eq!(Config::parse_i64("0").unwrap(), 0);
        assert_eq!(Config::parse_i64("1").unwrap(), 1);
        assert_eq!(Config::parse_i64("100").unwrap(), 100);
        assert_eq!(Config::parse_i64("-1").unwrap(), -1);
        assert_eq!(Config::parse_i64("-100").unwrap(), -100);
        assert_eq!(Config::parse_i64("1k").unwrap(), 1024);
        assert_eq!(Config::parse_i64("4k").unwrap(), 4096);
        assert_eq!(Config::parse_i64("1M").unwrap(), 1048576);
        assert_eq!(Config::parse_i64("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(Config::parse_i64("100G").unwrap(), 100 * 1024 * 1024 * 1024);
    }
}
