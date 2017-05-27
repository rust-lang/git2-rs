use std::ffi::CString;
use std::marker;
use std::path::{Path, PathBuf};
use std::ptr;
use std::str;
use libc;

use {raw, Error, ConfigLevel, Buf, IntoCString};
use util::{self, Binding};

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
pub struct ConfigEntries<'cfg> {
    raw: *mut raw::git_config_iterator,
    _marker: marker::PhantomData<&'cfg Config>,
}

impl Config {
    /// Allocate a new configuration object
    ///
    /// This object is empty, so you have to add a file to it before you can do
    /// anything with it.
    pub fn new() -> Result<Config, Error> {
        ::init();
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_config_new(&mut raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new config instance containing a single on-disk file
    pub fn open(path: &Path) -> Result<Config, Error> {
        ::init();
        let mut raw = ptr::null_mut();
        let path = try!(path.into_c_string());
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
        ::init();
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
    /// This method will not guess the path to the xdg compatible config file
    /// (`.config/git/config`).
    pub fn find_global() -> Result<PathBuf, Error> {
        ::init();
        let buf = Buf::new();
        unsafe { try_call!(raw::git_config_find_global(buf.raw())); }
        Ok(util::bytes2path(&buf).to_path_buf())
    }

    /// Locate the path to the system configuration file
    ///
    /// If /etc/gitconfig doesn't exist, it will look for %PROGRAMFILES%
    pub fn find_system() -> Result<PathBuf, Error> {
        ::init();
        let buf = Buf::new();
        unsafe { try_call!(raw::git_config_find_system(buf.raw())); }
        Ok(util::bytes2path(&buf).to_path_buf())
    }

    /// Locate the path to the global xdg compatible configuration file
    ///
    /// The xdg compatible configuration file is usually located in
    /// `$HOME/.config/git/config`.
    pub fn find_xdg() -> Result<PathBuf, Error> {
        ::init();
        let buf = Buf::new();
        unsafe { try_call!(raw::git_config_find_xdg(buf.raw())); }
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
    pub fn add_file(&mut self, path: &Path, level: ConfigLevel,
                    force: bool) -> Result<(), Error> {
        let path = try!(path.into_c_string());
        unsafe {
            try_call!(raw::git_config_add_file_ondisk(self.raw, path, level,
                                                      force));
            Ok(())
        }
    }

    /// Delete a config variable from the config file with the highest level
    /// (usually the local one).
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_delete_entry(self.raw, name));
            Ok(())
        }
    }

    /// Get the value of a boolean config variable.
    ///
    /// All config files will be looked into, in the order of their defined
    /// level. A higher level means a higher priority. The first occurrence of
    /// the variable will be returned here.
    pub fn get_bool(&self, name: &str) -> Result<bool, Error> {
        let mut out = 0 as libc::c_int;
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_get_bool(&mut out, &*self.raw, name));

        }
        Ok(if out == 0 {false} else {true})
    }

    /// Get the value of an integer config variable.
    ///
    /// All config files will be looked into, in the order of their defined
    /// level. A higher level means a higher priority. The first occurrence of
    /// the variable will be returned here.
    pub fn get_i32(&self, name: &str) -> Result<i32, Error> {
        let mut out = 0i32;
        let name = try!(CString::new(name));
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
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_get_int64(&mut out, &*self.raw, name));
        }
        Ok(out)
    }

    /// Get the value of a string config variable.
    ///
    /// This is the same as `get_bytes` except that it may return `Err` if
    /// the bytes are not valid utf-8.
    pub fn get_str(&self, name: &str) -> Result<&str, Error> {
        str::from_utf8(try!(self.get_bytes(name))).map_err(|_| {
            Error::from_str("configuration value is not valid utf8")
        })
    }

    /// Get the value of a string config variable as a byte slice.
    ///
    /// This method will return an error if this `Config` is not a snapshot.
    pub fn get_bytes(&self, name: &str) -> Result<&[u8], Error> {
        let mut ret = ptr::null();
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_get_string(&mut ret, &*self.raw, name));
            Ok(::opt_bytes(self, ret).unwrap())
        }
    }

    /// Get the value of a string config variable as an owned string.
    ///
    /// An error will be returned if the config value is not valid utf-8.
    pub fn get_string(&self, name: &str) -> Result<String, Error> {
        let ret = Buf::new();
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_get_string_buf(ret.raw(), self.raw, name));
        }
        str::from_utf8(&ret).map(|s| s.to_string()).map_err(|_| {
            Error::from_str("configuration value is not valid utf8")
        })
    }

    /// Get the value of a path config variable as an owned .
    pub fn get_path(&self, name: &str) -> Result<PathBuf, Error> {
        let ret = Buf::new();
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_get_path(ret.raw(), self.raw, name));
        }
        Ok(::util::bytes2path(&ret).to_path_buf())
    }

    /// Get the ConfigEntry for a config variable.
    pub fn get_entry(&self, name: &str) -> Result<ConfigEntry, Error> {
        let mut ret = ptr::null_mut();
        let name = try!(CString::new(name));
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
    /// # Example
    ///
    /// ```
    /// # #![allow(unstable)]
    /// use git2::Config;
    ///
    /// let cfg = Config::new().unwrap();
    ///
    /// for entry in &cfg.entries(None).unwrap() {
    ///     let entry = entry.unwrap();
    ///     println!("{} => {}", entry.name().unwrap(), entry.value().unwrap());
    /// }
    /// ```
    pub fn entries(&self, glob: Option<&str>) -> Result<ConfigEntries, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            match glob {
                Some(s) => {
                    let s = try!(CString::new(s));
                    try_call!(raw::git_config_iterator_glob_new(&mut ret,
                                                                &*self.raw,
                                                                s));
                }
                None => {
                    try_call!(raw::git_config_iterator_new(&mut ret, &*self.raw));
                }
            }
            Ok(Binding::from_raw(ret))
        }
    }

    /// Open the global/XDG configuration file according to git's rules
    ///
    /// Git allows you to store your global configuration at `$HOME/.config` or
    /// `$XDG_CONFIG_HOME/git/config`. For backwards compatability, the XDG file
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
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_set_bool(self.raw, name, value));
        }
        Ok(())
    }

    /// Set the value of an integer config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_i32(&mut self, name: &str, value: i32) -> Result<(), Error> {
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_set_int32(self.raw, name, value));
        }
        Ok(())
    }

    /// Set the value of an integer config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_i64(&mut self, name: &str, value: i64) -> Result<(), Error> {
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_config_set_int64(self.raw, name, value));
        }
        Ok(())
    }

    /// Set the value of an multivar config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_multivar(&mut self, name: &str, regexp: &str, value: &str) -> Result<(), Error> {
        let name = try!(CString::new(name));
        let regexp = try!(CString::new(regexp));
        let value = try!(CString::new(value));
        unsafe {
            try_call!(raw::git_config_set_multivar(self.raw, name, regexp, value));
        }
        Ok(())
    }

    /// Set the value of a string config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_str(&mut self, name: &str, value: &str) -> Result<(), Error> {
        let name = try!(CString::new(name));
        let value = try!(CString::new(value));
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
    /// Interprets "true", "yes", "on", 1, or any non-zero number as true.
    /// Interprets "false", "no", "off", 0, or an empty string as false.
    pub fn parse_bool<S: IntoCString>(s: S) -> Result<bool, Error> {
        let s = try!(s.into_c_string());
        let mut out = 0;
        ::init();
        unsafe {
            try_call!(raw::git_config_parse_bool(&mut out, s));
        }
        Ok(out != 0)
    }

    /// Parse a string as an i32; handles suffixes like k, M, or G, and
    /// multiplies by the appropriate power of 1024.
    pub fn parse_i32<S: IntoCString>(s: S) -> Result<i32, Error> {
        let s = try!(s.into_c_string());
        let mut out = 0;
        ::init();
        unsafe {
            try_call!(raw::git_config_parse_int32(&mut out, s));
        }
        Ok(out)
    }

    /// Parse a string as an i64; handles suffixes like k, M, or G, and
    /// multiplies by the appropriate power of 1024.
    pub fn parse_i64<S: IntoCString>(s: S) -> Result<i64, Error> {
        let s = try!(s.into_c_string());
        let mut out = 0;
        ::init();
        unsafe {
            try_call!(raw::git_config_parse_int64(&mut out, s));
        }
        Ok(out)
    }
}

impl Binding for Config {
    type Raw = *mut raw::git_config;
    unsafe fn from_raw(raw: *mut raw::git_config) -> Config {
        Config { raw: raw }
    }
    fn raw(&self) -> *mut raw::git_config { self.raw }
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
    pub fn name(&self) -> Option<&str> { str::from_utf8(self.name_bytes()).ok() }

    /// Gets the name of this entry as a byte slice.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, (*self.raw).name).unwrap() }
    }

    /// Gets the value of this entry.
    ///
    /// May return `None` if the value is not valid utf-8
    pub fn value(&self) -> Option<&str> { str::from_utf8(self.value_bytes()).ok() }

    /// Gets the value of this entry as a byte slice.
    pub fn value_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, (*self.raw).value).unwrap() }
    }

    /// Gets the configuration level of this entry.
    pub fn level(&self) -> ConfigLevel {
        unsafe { ConfigLevel::from_raw((*self.raw).level) }
    }
}

impl<'cfg> Binding for ConfigEntry<'cfg> {
    type Raw = *mut raw::git_config_entry;

    unsafe fn from_raw(raw: *mut raw::git_config_entry)
                           -> ConfigEntry<'cfg> {
        ConfigEntry {
            raw: raw,
            _marker: marker::PhantomData,
            owned: true,
        }
    }
    fn raw(&self) -> *mut raw::git_config_entry { self.raw }
}

impl<'cfg> Binding for ConfigEntries<'cfg> {
    type Raw = *mut raw::git_config_iterator;

    unsafe fn from_raw(raw: *mut raw::git_config_iterator)
                           -> ConfigEntries<'cfg> {
        ConfigEntries {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_config_iterator { self.raw }
}

// entries are only valid until the iterator is freed, so this impl is for
// `&'b T` instead of `T` to have a lifetime to tie them to.
//
// It's also not implemented for `&'b mut T` so we can have multiple entries
// (ok).
impl<'cfg, 'b> Iterator for &'b ConfigEntries<'cfg> {
    type Item = Result<ConfigEntry<'b>, Error>;
    fn next(&mut self) -> Option<Result<ConfigEntry<'b>, Error>> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call_iter!(raw::git_config_next(&mut raw, self.raw));
            Some(Ok(ConfigEntry {
                owned: false,
                raw: raw,
                _marker: marker::PhantomData,
            }))
        }
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
    use tempdir::TempDir;

    use Config;

    #[test]
    fn smoke() {
        let _cfg = Config::new().unwrap();
        let _ = Config::find_global();
        let _ = Config::find_system();
        let _ = Config::find_xdg();
    }

    #[test]
    fn persisted() {
        let td = TempDir::new("test").unwrap();
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

        for entry in &cfg.entries(None).unwrap() {
            let entry = entry.unwrap();
            entry.name();
            entry.value();
            entry.level();
        }
    }

    #[test]
    fn multivar() {
        let td = TempDir::new("test").unwrap();
        let path = td.path().join("foo");
        File::create(&path).unwrap();

        let mut cfg = Config::open(&path).unwrap();
        cfg.set_multivar("foo.bar", "^$", "baz").unwrap();
        cfg.set_multivar("foo.bar", "^$", "qux").unwrap();

        let mut values: Vec<String> = cfg.entries(None)
            .unwrap()
            .into_iter()
            .map(|entry| entry.unwrap().value().unwrap().into())
            .collect();
        values.sort();
        assert_eq!(values, ["baz", "qux"]);
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
        assert_eq!(Config::parse_i32("1G").unwrap(), 1024*1024*1024);

        assert_eq!(Config::parse_i64("0").unwrap(), 0);
        assert_eq!(Config::parse_i64("1").unwrap(), 1);
        assert_eq!(Config::parse_i64("100").unwrap(), 100);
        assert_eq!(Config::parse_i64("-1").unwrap(), -1);
        assert_eq!(Config::parse_i64("-100").unwrap(), -100);
        assert_eq!(Config::parse_i64("1k").unwrap(), 1024);
        assert_eq!(Config::parse_i64("4k").unwrap(), 4096);
        assert_eq!(Config::parse_i64("1M").unwrap(), 1048576);
        assert_eq!(Config::parse_i64("1G").unwrap(), 1024*1024*1024);
        assert_eq!(Config::parse_i64("100G").unwrap(), 100*1024*1024*1024);
    }
}
