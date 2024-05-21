use log::{debug, trace};
use std::ffi::CString;
use std::io::Write;
use std::mem;
use std::path::Path;
use std::process::{Command, Stdio};
use std::ptr;

use crate::util::Binding;
use crate::{raw, Config, Error, IntoCString};

/// A structure to represent git credentials in libgit2.
pub struct Cred {
    raw: *mut raw::git_cred,
}

/// Management of the gitcredentials(7) interface.
pub struct CredentialHelper {
    /// A public field representing the currently discovered username from
    /// configuration.
    pub username: Option<String>,
    protocol: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    path: Option<String>,
    url: String,
    commands: Vec<String>,
}

impl Cred {
    /// Create a "default" credential usable for Negotiate mechanisms like NTLM
    /// or Kerberos authentication.
    pub fn default() -> Result<Cred, Error> {
        crate::init();
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_cred_default_new(&mut out));
            Ok(Binding::from_raw(out))
        }
    }

    /// Create a new ssh key credential object used for querying an ssh-agent.
    ///
    /// The username specified is the username to authenticate.
    pub fn ssh_key_from_agent(username: &str) -> Result<Cred, Error> {
        crate::init();
        let mut out = ptr::null_mut();
        let username = CString::new(username)?;
        unsafe {
            try_call!(raw::git_cred_ssh_key_from_agent(&mut out, username));
            Ok(Binding::from_raw(out))
        }
    }

    /// Create a new passphrase-protected ssh key credential object.
    pub fn ssh_key(
        username: &str,
        publickey: Option<&Path>,
        privatekey: &Path,
        passphrase: Option<&str>,
    ) -> Result<Cred, Error> {
        crate::init();
        let username = CString::new(username)?;
        let publickey = crate::opt_cstr(publickey)?;
        let privatekey = privatekey.into_c_string()?;
        let passphrase = crate::opt_cstr(passphrase)?;
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_cred_ssh_key_new(
                &mut out, username, publickey, privatekey, passphrase
            ));
            Ok(Binding::from_raw(out))
        }
    }

    /// Create a new ssh key credential object reading the keys from memory.
    pub fn ssh_key_from_memory(
        username: &str,
        publickey: Option<&str>,
        privatekey: &str,
        passphrase: Option<&str>,
    ) -> Result<Cred, Error> {
        crate::init();
        let username = CString::new(username)?;
        let publickey = crate::opt_cstr(publickey)?;
        let privatekey = CString::new(privatekey)?;
        let passphrase = crate::opt_cstr(passphrase)?;
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_cred_ssh_key_memory_new(
                &mut out, username, publickey, privatekey, passphrase
            ));
            Ok(Binding::from_raw(out))
        }
    }

    /// Create a new plain-text username and password credential object.
    pub fn userpass_plaintext(username: &str, password: &str) -> Result<Cred, Error> {
        crate::init();
        let username = CString::new(username)?;
        let password = CString::new(password)?;
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_cred_userpass_plaintext_new(
                &mut out, username, password
            ));
            Ok(Binding::from_raw(out))
        }
    }

    /// Attempt to read `credential.helper` according to gitcredentials(7) [1]
    ///
    /// This function will attempt to parse the user's `credential.helper`
    /// configuration, invoke the necessary processes, and read off what the
    /// username/password should be for a particular URL.
    ///
    /// The returned credential type will be a username/password credential if
    /// successful.
    ///
    /// [1]: https://www.kernel.org/pub/software/scm/git/docs/gitcredentials.html
    pub fn credential_helper(
        config: &Config,
        url: &str,
        username: Option<&str>,
    ) -> Result<Cred, Error> {
        match CredentialHelper::new(url)
            .config(config)
            .username(username)
            .execute()
        {
            Some((username, password)) => Cred::userpass_plaintext(&username, &password),
            None => Err(Error::from_str(
                "failed to acquire username/password \
                 from local configuration",
            )),
        }
    }

    /// Create a credential to specify a username.
    ///
    /// This is used with ssh authentication to query for the username if none is
    /// specified in the URL.
    pub fn username(username: &str) -> Result<Cred, Error> {
        crate::init();
        let username = CString::new(username)?;
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_cred_username_new(&mut out, username));
            Ok(Binding::from_raw(out))
        }
    }

    /// Check whether a credential object contains username information.
    pub fn has_username(&self) -> bool {
        unsafe { raw::git_cred_has_username(self.raw) == 1 }
    }

    /// Return the type of credentials that this object represents.
    pub fn credtype(&self) -> raw::git_credtype_t {
        unsafe { (*self.raw).credtype }
    }

    /// Unwrap access to the underlying raw pointer, canceling the destructor
    pub unsafe fn unwrap(mut self) -> *mut raw::git_cred {
        mem::replace(&mut self.raw, ptr::null_mut())
    }
}

impl Binding for Cred {
    type Raw = *mut raw::git_cred;

    unsafe fn from_raw(raw: *mut raw::git_cred) -> Cred {
        Cred { raw }
    }
    fn raw(&self) -> *mut raw::git_cred {
        self.raw
    }
}

impl Drop for Cred {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                if let Some(f) = (*self.raw).free {
                    f(self.raw)
                }
            }
        }
    }
}

impl CredentialHelper {
    /// Create a new credential helper object which will be used to probe git's
    /// local credential configuration.
    ///
    /// The URL specified is the namespace on which this will query credentials.
    /// Invalid URLs are currently ignored.
    pub fn new(url: &str) -> CredentialHelper {
        let mut ret = CredentialHelper {
            protocol: None,
            host: None,
            port: None,
            path: None,
            username: None,
            url: url.to_string(),
            commands: Vec::new(),
        };

        // Parse out the (protocol, host) if one is available
        if let Ok(url) = url::Url::parse(url) {
            if let Some(url::Host::Domain(s)) = url.host() {
                ret.host = Some(s.to_string());
            }
            ret.port = url.port();
            ret.protocol = Some(url.scheme().to_string());
        }
        ret
    }

    /// Set the username that this credential helper will query with.
    ///
    /// By default the username is `None`.
    pub fn username(&mut self, username: Option<&str>) -> &mut CredentialHelper {
        self.username = username.map(|s| s.to_string());
        self
    }

    /// Query the specified configuration object to discover commands to
    /// execute, usernames to query, etc.
    pub fn config(&mut self, config: &Config) -> &mut CredentialHelper {
        // Figure out the configured username/helper program.
        //
        // see http://git-scm.com/docs/gitcredentials.html#_configuration_options
        if self.username.is_none() {
            self.config_username(config);
        }
        self.config_helper(config);
        self.config_use_http_path(config);
        self
    }

    // Configure the queried username from `config`
    fn config_username(&mut self, config: &Config) {
        let key = self.exact_key("username");
        self.username = config
            .get_string(&key)
            .ok()
            .or_else(|| {
                self.url_key("username")
                    .and_then(|s| config.get_string(&s).ok())
            })
            .or_else(|| config.get_string("credential.username").ok())
    }

    // Discover all `helper` directives from `config`
    fn config_helper(&mut self, config: &Config) {
        let exact = config.get_string(&self.exact_key("helper"));
        self.add_command(exact.as_ref().ok().map(|s| &s[..]));
        if let Some(key) = self.url_key("helper") {
            let url = config.get_string(&key);
            self.add_command(url.as_ref().ok().map(|s| &s[..]));
        }
        let global = config.get_string("credential.helper");
        self.add_command(global.as_ref().ok().map(|s| &s[..]));
    }

    // Discover `useHttpPath` from `config`
    fn config_use_http_path(&mut self, config: &Config) {
        let mut use_http_path = false;
        if let Some(value) = config.get_bool(&self.exact_key("useHttpPath")).ok() {
            use_http_path = value;
        } else if let Some(value) = self
            .url_key("useHttpPath")
            .and_then(|key| config.get_bool(&key).ok())
        {
            use_http_path = value;
        } else if let Some(value) = config.get_bool("credential.useHttpPath").ok() {
            use_http_path = value;
        }

        if use_http_path {
            if let Ok(url) = url::Url::parse(&self.url) {
                let path = url.path();
                // Url::parse always includes a leading slash for rooted URLs, while git does not.
                self.path = Some(path.strip_prefix('/').unwrap_or(path).to_string());
            }
        }
    }

    // Add a `helper` configured command to the list of commands to execute.
    //
    // see https://www.kernel.org/pub/software/scm/git/docs/technical
    //                           /api-credentials.html#_credential_helpers
    fn add_command(&mut self, cmd: Option<&str>) {
        let cmd = match cmd {
            Some("") | None => return,
            Some(s) => s,
        };

        if cmd.starts_with('!') {
            self.commands.push(cmd[1..].to_string());
        } else if cmd.contains("/") || cmd.contains("\\") {
            self.commands.push(cmd.to_string());
        } else {
            self.commands.push(format!("git credential-{}", cmd));
        }
    }

    fn exact_key(&self, name: &str) -> String {
        format!("credential.{}.{}", self.url, name)
    }

    fn url_key(&self, name: &str) -> Option<String> {
        match (&self.host, &self.protocol) {
            (&Some(ref host), &Some(ref protocol)) => {
                Some(format!("credential.{}://{}.{}", protocol, host, name))
            }
            _ => None,
        }
    }

    /// Execute this helper, attempting to discover a username/password pair.
    ///
    /// All I/O errors are ignored, (to match git behavior), and this function
    /// only succeeds if both a username and a password were found
    pub fn execute(&self) -> Option<(String, String)> {
        let mut username = self.username.clone();
        let mut password = None;
        for cmd in &self.commands {
            let (u, p) = self.execute_cmd(cmd, &username);
            if u.is_some() && username.is_none() {
                username = u;
            }
            if p.is_some() && password.is_none() {
                password = p;
            }
            if username.is_some() && password.is_some() {
                break;
            }
        }

        match (username, password) {
            (Some(u), Some(p)) => Some((u, p)),
            _ => None,
        }
    }

    // Execute the given `cmd`, providing the appropriate variables on stdin and
    // then afterwards parsing the output into the username/password on stdout.
    fn execute_cmd(
        &self,
        cmd: &str,
        username: &Option<String>,
    ) -> (Option<String>, Option<String>) {
        macro_rules! my_try( ($e:expr) => (
            match $e {
                Ok(e) => e,
                Err(e) => {
                    debug!("{} failed with {}", stringify!($e), e);
                    return (None, None)
                }
            }
        ) );

        // It looks like the `cmd` specification is typically bourne-shell-like
        // syntax, so try that first. If that fails, though, we may be on a
        // Windows machine for example where `sh` isn't actually available by
        // default. Most credential helper configurations though are pretty
        // simple (aka one or two space-separated strings) so also try to invoke
        // the process directly.
        //
        // If that fails then it's up to the user to put `sh` in path and make
        // sure it works.
        let mut c = Command::new("sh");
        c.arg("-c")
            .arg(&format!("{} get", cmd))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        debug!("executing credential helper {:?}", c);
        let mut p = match c.spawn() {
            Ok(p) => p,
            Err(e) => {
                debug!("`sh` failed to spawn: {}", e);
                let mut parts = cmd.split_whitespace();
                let mut c = Command::new(parts.next().unwrap());
                for arg in parts {
                    c.arg(arg);
                }
                c.arg("get")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                debug!("executing credential helper {:?}", c);
                match c.spawn() {
                    Ok(p) => p,
                    Err(e) => {
                        debug!("fallback of {:?} failed with {}", cmd, e);
                        return (None, None);
                    }
                }
            }
        };

        // Ignore write errors as the command may not actually be listening for
        // stdin
        {
            let stdin = p.stdin.as_mut().unwrap();
            if let Some(ref p) = self.protocol {
                let _ = writeln!(stdin, "protocol={}", p);
            }
            if let Some(ref p) = self.host {
                if let Some(ref p2) = self.port {
                    let _ = writeln!(stdin, "host={}:{}", p, p2);
                } else {
                    let _ = writeln!(stdin, "host={}", p);
                }
            }
            if let Some(ref p) = self.path {
                let _ = writeln!(stdin, "path={}", p);
            }
            if let Some(ref p) = *username {
                let _ = writeln!(stdin, "username={}", p);
            }
        }
        let output = my_try!(p.wait_with_output());
        if !output.status.success() {
            debug!(
                "credential helper failed: {}\nstdout ---\n{}\nstderr ---\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return (None, None);
        }
        trace!(
            "credential helper stderr ---\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.parse_output(output.stdout)
    }

    // Parse the output of a command into the username/password found
    fn parse_output(&self, output: Vec<u8>) -> (Option<String>, Option<String>) {
        // Parse the output of the command, looking for username/password
        let mut username = None;
        let mut password = None;
        for line in output.split(|t| *t == b'\n') {
            let mut parts = line.splitn(2, |t| *t == b'=');
            let key = parts.next().unwrap();
            let value = match parts.next() {
                Some(s) => s,
                None => {
                    trace!("ignoring output line: {}", String::from_utf8_lossy(line));
                    continue;
                }
            };
            let value = match String::from_utf8(value.to_vec()) {
                Ok(s) => s,
                Err(..) => continue,
            };
            match key {
                b"username" => username = Some(value),
                b"password" => password = Some(value),
                _ => {}
            }
        }
        (username, password)
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    use tempfile::TempDir;

    use crate::{Config, ConfigLevel, Cred, CredentialHelper};

    macro_rules! test_cfg( ($($k:expr => $v:expr),*) => ({
        let td = TempDir::new().unwrap();
        let mut cfg = Config::new().unwrap();
        cfg.add_file(&td.path().join("cfg"), ConfigLevel::App, false).unwrap();
        $(cfg.set_str($k, $v).unwrap();)*
        cfg
    }) );

    #[test]
    fn smoke() {
        Cred::default().unwrap();
    }

    #[test]
    fn credential_helper1() {
        let cfg = test_cfg! {
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .unwrap();
        assert_eq!(u, "a");
        assert_eq!(p, "b");
    }

    #[test]
    fn credential_helper2() {
        let cfg = test_cfg! {};
        assert!(CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .is_none());
    }

    #[test]
    fn credential_helper3() {
        let cfg = test_cfg! {
            "credential.https://example.com.helper" =>
                    "!f() { echo username=c; }; f",
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .unwrap();
        assert_eq!(u, "c");
        assert_eq!(p, "b");
    }

    #[test]
    fn credential_helper4() {
        if cfg!(windows) {
            return;
        } // shell scripts don't work on Windows

        let td = TempDir::new().unwrap();
        let path = td.path().join("script");
        File::create(&path)
            .unwrap()
            .write(
                br"\
#!/bin/sh
echo username=c
",
            )
            .unwrap();
        chmod(&path);
        let cfg = test_cfg! {
            "credential.https://example.com.helper" =>
                    &path.display().to_string()[..],
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .unwrap();
        assert_eq!(u, "c");
        assert_eq!(p, "b");
    }

    #[test]
    fn credential_helper5() {
        if cfg!(windows) {
            return;
        } // shell scripts don't work on Windows
        let td = TempDir::new().unwrap();
        let path = td.path().join("git-credential-script");
        File::create(&path)
            .unwrap()
            .write(
                br"\
#!/bin/sh
echo username=c
",
            )
            .unwrap();
        chmod(&path);

        let paths = env::var("PATH").unwrap();
        let paths =
            env::split_paths(&paths).chain(path.parent().map(|p| p.to_path_buf()).into_iter());
        env::set_var("PATH", &env::join_paths(paths).unwrap());

        let cfg = test_cfg! {
            "credential.https://example.com.helper" => "script",
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .unwrap();
        assert_eq!(u, "c");
        assert_eq!(p, "b");
    }

    #[test]
    fn credential_helper6() {
        let cfg = test_cfg! {
            "credential.helper" => ""
        };
        assert!(CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .is_none());
    }

    #[test]
    fn credential_helper7() {
        if cfg!(windows) {
            return;
        } // shell scripts don't work on Windows
        let td = TempDir::new().unwrap();
        let path = td.path().join("script");
        File::create(&path)
            .unwrap()
            .write(
                br"\
#!/bin/sh
echo username=$1
echo password=$2
",
            )
            .unwrap();
        chmod(&path);
        let cfg = test_cfg! {
            "credential.helper" => &format!("{} a b", path.display())
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
            .config(&cfg)
            .execute()
            .unwrap();
        assert_eq!(u, "a");
        assert_eq!(p, "b");
    }

    #[test]
    fn credential_helper8() {
        let cfg = test_cfg! {
            "credential.useHttpPath" => "true"
        };
        let mut helper = CredentialHelper::new("https://example.com/foo/bar");
        helper.config(&cfg);
        assert_eq!(helper.path.as_deref(), Some("foo/bar"));
    }

    #[test]
    fn credential_helper9() {
        let cfg = test_cfg! {
            "credential.helper" => "!f() { while read line; do eval $line; done; if [ \"$host\" = example.com:3000 ]; then echo username=a; echo password=b; fi; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com:3000/foo/bar")
            .config(&cfg)
            .execute()
            .unwrap();
        assert_eq!(u, "a");
        assert_eq!(p, "b");
    }

    #[test]
    #[cfg(feature = "ssh")]
    fn ssh_key_from_memory() {
        let cred = Cred::ssh_key_from_memory(
            "test",
            Some("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQDByAO8uj+kXicj6C2ODMspgmUoVyl5eaw8vR6a1yEnFuJFzevabNlN6Ut+CPT3TRnYk5BW73pyXBtnSL2X95BOnbjMDXc4YIkgs3YYHWnxbqsD4Pj/RoGqhf+gwhOBtL0poh8tT8WqXZYxdJQKLQC7oBqf3ykCEYulE4oeRUmNh4IzEE+skD/zDkaJ+S1HRD8D8YCiTO01qQnSmoDFdmIZTi8MS8Cw+O/Qhym1271ThMlhD6PubSYJXfE6rVbE7A9RzH73A6MmKBlzK8VTb4SlNSrr/DOk+L0uq+wPkv+pm+D9WtxoqQ9yl6FaK1cPawa3+7yRNle3m+72KCtyMkQv"),
            r#"
                -----BEGIN RSA PRIVATE KEY-----
                Proc-Type: 4,ENCRYPTED
                DEK-Info: AES-128-CBC,818C7722D3B01F2161C2ACF6A5BBAAE8

                3Cht4QB3PcoQ0I55j1B3m2ZzIC/mrh+K5nQeA1Vy2GBTMyM7yqGHqTOv7qLhJscd
                H+cB0Pm6yCr3lYuNrcKWOCUto+91P7ikyARruHVwyIxKdNx15uNulOzQJHQWNbA4
                RQHlhjON4atVo2FyJ6n+ujK6QiBg2PR5Vbbw/AtV6zBCFW3PhzDn+qqmHjpBFqj2
                vZUUe+MkDQcaF5J45XMHahhSdo/uKCDhfbylExp/+ACWkvxdPpsvcARM6X434ucD
                aPY+4i0/JyLkdbm0GFN9/q3i53qf4kCBhojFl4AYJdGI0AzAgbdTXZ7EJHbAGZHS
                os5K0oTwDVXMI0sSE2I/qHxaZZsDP1dOKq6di6SFPUp8liYimm7rNintRX88Gl2L
                g1ko9abp/NlgD0YY/3mad+NNAISDL/YfXq2fklH3En3/7ZrOVZFKfZXwQwas5g+p
                VQPKi3+ae74iOjLyuPDSc1ePmhUNYeP+9rLSc0wiaiHqls+2blPPDxAGMEo63kbz
                YPVjdmuVX4VWnyEsfTxxJdFDYGSNh6rlrrO1RFrex7kJvpg5gTX4M/FT8TfCd7Hn
                M6adXsLMqwu5tz8FuDmAtVdq8zdSrgZeAbpJ9D3EDOmZ70xz4XBL19ImxDp+Qqs2
                kQX7kobRzeeP2URfRoGr7XZikQWyQ2UASfPcQULY8R58QoZWWsQ4w51GZHg7TDnw
                1DRo/0OgkK7Gqf215nFmMpB4uyi58cq3WFwWQa1IqslkObpVgBQZcNZb/hKUYPGk
                g4zehfIgAfCdnQHwZvQ6Fdzhcs3SZeO+zVyuiZN3Gsi9HU0/1vpAKiuuOzcG02vF
                b6Y6hwsAA9yphF3atI+ARD4ZwXdDfzuGb3yJglMT3Fr/xuLwAvdchRo1spANKA0E
                tT5okLrK0H4wnHvf2SniVVWRhmJis0lQo9LjGGwRIdsPpVnJSDvaISIVF+fHT90r
                HvxN8zXI93x9jcPtwp7puQ1C7ehKJK10sZ71OLIZeuUgwt+5DRunqg6evPco9Go7
                UOGwcVhLY200KT+1k7zWzCS0yVQp2HRm6cxsZXAp4ClBSwIx15eIoLIrjZdJRjCq
                COp6pZx1fnvJ9ERIvl5hon+Ty+renMcFKz2HmchC7egpcqIxW9Dsv6zjhHle6pxb
                37GaEKHF2KA3RN+dSV/K8n+C9Yent5tx5Y9a/pMcgRGtgu+G+nyFmkPKn5Zt39yX
                qDpyM0LtbRVZPs+MgiqoGIwYc/ujoCq7GL38gezsBQoHaTt79yYBqCp6UR0LMuZ5
                f/7CtWqffgySfJ/0wjGidDAumDv8CK45AURpL/Z+tbFG3M9ar/LZz/Y6EyBcLtGY
                Wwb4zs8zXIA0qHrjNTnPqHDvezziArYfgPjxCIHMZzms9Yn8+N02p39uIytqg434
                BAlCqZ7GYdDFfTpWIwX+segTK9ux0KdBqcQv+9Fwwjkq9KySnRKqNl7ZJcefFZJq
                c6PA1iinZWBjuaO1HKx3PFulrl0bcpR9Kud1ZIyfnh5rwYN8UQkkcR/wZPla04TY
                8l5dq/LI/3G5sZXwUHKOcuQWTj7Saq7Q6gkKoMfqt0wC5bpZ1m17GHPoMz6GtX9O
                -----END RSA PRIVATE KEY-----
            "#,
            Some("test123"));
        assert!(cred.is_ok());
    }

    #[cfg(unix)]
    fn chmod(path: &Path) {
        use std::fs;
        use std::os::unix::prelude::*;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    #[cfg(windows)]
    fn chmod(_path: &Path) {}
}
