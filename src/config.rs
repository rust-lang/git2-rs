use std::kinds::marker;
use std::str;
use libc;

use {raw, Error, ConfigLevel, Buf};

/// A structure representing a git configuration key/value store
pub struct Config {
    raw: *mut raw::git_config,
    marker: marker::NoSync,
}

/// A struct representing a certain entry owned by a `Config` instance.
///
/// An entry has a name, a value, and a level it applies to.
pub struct ConfigEntry<'cfg> {
    raw: *const raw::git_config_entry,
    marker1: marker::ContravariantLifetime<'cfg>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

/// An iterator over the `ConfigEntry` values of a `Config` structure.
pub struct ConfigEntries<'cfg> {
    raw: *mut raw::git_config_iterator,
    marker1: marker::ContravariantLifetime<'cfg>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl Config {
    /// Allocate a new configuration object
    ///
    /// This object is empty, so you have to add a file to it before you can do
    /// anything with it.
    pub fn new() -> Result<Config, Error> {
        ::init();
        let mut raw = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_config_new(&mut raw));
            Ok(Config::from_raw(raw))
        }
    }

    /// Create a new config instance containing a single on-disk file
    pub fn open(path: &Path) -> Result<Config, Error> {
        ::init();
        let mut raw = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_config_open_ondisk(&mut raw, path.to_c_str()));
            Ok(Config::from_raw(raw))
        }
    }

    /// Open the global, XDG and system configuration files
    ///
    /// Utility wrapper that finds the global, XDG and system configuration
    /// files and opens them into a single prioritized config object that can
    /// be used when accessing default config data outside a repository.
    pub fn open_default() -> Result<Config, Error> {
        ::init();
        let mut raw = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_config_open_default(&mut raw));
            Ok(Config::from_raw(raw))
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
    pub fn find_global() -> Result<Path, Error> {
        ::init();
        let mut buf = Buf::new();
        unsafe { try_call!(raw::git_config_find_global(buf.raw())); }
        Ok(Path::new(buf.get()))
    }

    /// Locate the path to the system configuration file
    ///
    /// If /etc/gitconfig doesn't exist, it will look for %PROGRAMFILES%
    pub fn find_system() -> Result<Path, Error> {
        ::init();
        let mut buf = Buf::new();
        unsafe { try_call!(raw::git_config_find_system(buf.raw())); }
        Ok(Path::new(buf.get()))
    }

    /// Locate the path to the global xdg compatible configuration file
    ///
    /// The xdg compatible configuration file is usually located in
    /// `$HOME/.config/git/config`.
    pub fn find_xdg() -> Result<Path, Error> {
        ::init();
        let mut buf = Buf::new();
        unsafe { try_call!(raw::git_config_find_xdg(buf.raw())); }
        Ok(Path::new(buf.get()))
    }

    /// Consumes ownership of a raw config pointer, returning a wrapped object.
    ///
    /// This function is unsafe as the validity of `raw` cannot be guaranteed.
    pub unsafe fn from_raw(raw: *mut raw::git_config) -> Config {
        Config { raw: raw, marker: marker::NoSync }
    }

    /// Gain access to the underlying raw pointer of this config
    pub fn raw(&self) -> *mut raw::git_config { self.raw }

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
        unsafe {
            try_call!(raw::git_config_add_file_ondisk(self.raw, path.to_c_str(),
                                                      level, force));
            Ok(())
        }
    }

    /// Delete a config variable from the config file with the highest level
    /// (usually the local one).
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_config_delete_entry(self.raw, name.to_c_str()));
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
        unsafe {
            try_call!(raw::git_config_get_bool(&mut out, &*self.raw,
                                               name.to_c_str()));
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
        unsafe {
            try_call!(raw::git_config_get_int32(&mut out, &*self.raw,
                                                name.to_c_str()));
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
        unsafe {
            try_call!(raw::git_config_get_int64(&mut out, &*self.raw,
                                                name.to_c_str()));
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
    pub fn get_bytes(&self, name: &str) -> Result<&[u8], Error> {
        let mut ret = 0 as *const libc::c_char;
        unsafe {
            try_call!(raw::git_config_get_string(&mut ret, &*self.raw,
                                                 name.to_c_str()));
            Ok(::opt_bytes(self, ret).unwrap())
        }
    }

    /// Get the ConfigEntry for a config variable.
    pub fn get_entry(&self, name: &str) -> Result<ConfigEntry, Error> {
        let mut ret = 0 as *const raw::git_config_entry;
        unsafe {
            try_call!(raw::git_config_get_entry(&mut ret, &*self.raw,
                                                name.to_c_str()));
            Ok(ConfigEntry::from_raw(ret))
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
    /// use git2::Config;
    ///
    /// let cfg = Config::new().unwrap();
    ///
    /// for entry in &cfg.entries(None).unwrap() {
    ///     println!("{} => {}", entry.name(), entry.value());
    /// }
    /// ```
    pub fn entries(&self, glob: Option<&str>) -> Result<ConfigEntries, Error> {
        let mut ret = 0 as *mut raw::git_config_iterator;
        unsafe {
            match glob {
                Some(s) => {
                    try_call!(raw::git_config_iterator_glob_new(&mut ret,
                                                                &*self.raw,
                                                                s.to_c_str()));
                }
                None => {
                    try_call!(raw::git_config_iterator_new(&mut ret, &*self.raw));
                }
            }
            Ok(ConfigEntries::from_raw(ret))
        }
    }

    /// Open the global/XDG configuration file according to git's rules
    ///
    /// Git allows you to store your global configuration at `$HOME/.config` or
    /// `$XDG_CONFIG_HOME/git/config`. For backwards compatability, the XDG file
    /// shouldn't be used unless the use has created it explicitly. With this
    /// function you'll open the correct one to write to.
    pub fn open_global(&mut self) -> Result<Config, Error> {
        let mut raw = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_config_open_global(&mut raw, self.raw));
            Ok(Config::from_raw(raw))
        }
    }

    /// Build a single-level focused config object from a multi-level one.
    ///
    /// The returned config object can be used to perform get/set/delete
    /// operations on a single specific level.
    pub fn open_level(&self, level: ConfigLevel) -> Result<Config, Error> {
        let mut raw = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_config_open_level(&mut raw, &*self.raw, level));
            Ok(Config::from_raw(raw))
        }
    }

    /// Set the value of a boolean config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_bool(&mut self, name: &str, value: bool) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_config_set_bool(self.raw, name.to_c_str(),
                                               value));
        }
        Ok(())
    }

    /// Set the value of an integer config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_i32(&mut self, name: &str, value: i32) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_config_set_int32(self.raw, name.to_c_str(),
                                                value));
        }
        Ok(())
    }

    /// Set the value of an integer config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_i64(&mut self, name: &str, value: i64) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_config_set_int64(self.raw, name.to_c_str(),
                                                value));
        }
        Ok(())
    }

    /// Set the value of a string config variable in the config file with the
    /// highest level (usually the local one).
    pub fn set_str(&mut self, name: &str, value: &str) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_config_set_string(self.raw, name.to_c_str(),
                                                 value.to_c_str()));
        }
        Ok(())
    }

    /// Create a snapshot of the configuration
    ///
    /// Create a snapshot of the current state of a configuration, which allows
    /// you to look into a consistent view of the configuration for looking up
    /// complex values (e.g. a remote, submodule).
    pub fn snapshot(&mut self) -> Result<Config, Error> {
        let mut ret = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_config_snapshot(&mut ret, self.raw));
            Ok(Config::from_raw(ret))
        }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        unsafe { raw::git_config_free(self.raw) }
    }
}

impl<'cfg> ConfigEntry<'cfg> {
    /// Creates a new config entry from the raw components.
    ///
    /// This method is unsafe as the validity of `raw` is not guaranteed.
    pub unsafe fn from_raw(raw: *const raw::git_config_entry)
                           -> ConfigEntry<'cfg> {
        ConfigEntry {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

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

impl<'cfg> ConfigEntries<'cfg> {
    /// Creates a new iterator from its raw component.
    ///
    /// This function is unsafe as the validity of `raw` is not guaranteed.
    pub unsafe fn from_raw(raw: *mut raw::git_config_iterator)
                           -> ConfigEntries<'cfg> {
        ConfigEntries {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }
}

// entries are only valid until the iterator is freed, so this impl is for
// `&'b T` instead of `T` to have a lifetime to tie them to.
//
// It's also not implemented for `&'b mut T` so we can have multiple entries
// (ok).
impl<'cfg, 'b> Iterator<ConfigEntry<'b>> for &'b ConfigEntries<'cfg> {
    fn next(&mut self) -> Option<ConfigEntry<'b>> {
        let mut raw = 0 as *mut raw::git_config_entry;
        unsafe {
            if raw::git_config_next(&mut raw, self.raw) == 0 {
                Some(ConfigEntry::from_raw(raw))
            } else {
                None
            }
        }
    }
}

#[unsafe_destructor]
impl<'cfg> Drop for ConfigEntries<'cfg> {
    fn drop(&mut self) {
        unsafe { raw::git_config_iterator_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{TempDir, File};
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

        let cfg = Config::open(&path).unwrap();
        assert_eq!(cfg.get_bool("foo.k1").unwrap(), true);
        assert_eq!(cfg.get_i32("foo.k2").unwrap(), 1);
        assert_eq!(cfg.get_i64("foo.k3").unwrap(), 2);
        assert_eq!(cfg.get_str("foo.k4").unwrap(), "bar");

        for entry in &cfg.entries(None).unwrap() {
            entry.name();
            entry.value();
            entry.level();
        }
    }
}
