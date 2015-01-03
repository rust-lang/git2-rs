use std::c_str::ToCStr;
use std::mem;
use std::io::Command;
use url::{self, UrlParser};

use {raw, Error, Config};

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
    url: String,
    commands: Vec<String>,
}

impl Cred {
    /// Create a new credential object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_cred) -> Cred {
        Cred { raw: raw }
    }

    /// Create a "default" credential usable for Negotiate mechanisms like NTLM
    /// or Kerberos authentication.
    pub fn default() -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_default_new(&mut out));
            Ok(Cred::from_raw(out))
        }
    }

    /// Create a new ssh key credential object used for querying an ssh-agent.
    ///
    /// The username specified is the username to authenticate.
    pub fn ssh_key_from_agent(username: &str) -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_ssh_key_from_agent(&mut out,
                                                       username.to_c_str()));
            Ok(Cred::from_raw(out))
        }
    }

    /// Create a new passphrase-protected ssh key credential object.
    pub fn ssh_key(username: &str,
                   publickey: Option<&Path>,
                   privatekey: &Path,
                   passphrase: Option<&str>) -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_ssh_key_new(&mut out,
                                                username.to_c_str(),
                                                publickey.map(|s| s.to_c_str()),
                                                privatekey.to_c_str(),
                                                passphrase.map(|s| s.to_c_str())));
            Ok(Cred::from_raw(out))
        }
    }

    /// Create a new plain-text username and password credential object.
    pub fn userpass_plaintext(username: &str,
                              password: &str) -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_userpass_plaintext_new(&mut out,
                                                           username.to_c_str(),
                                                           password.to_c_str()));
            Ok(Cred::from_raw(out))
        }
    }

    /// Attempt to read `credential.helper` according to gitcredentials(7) [1]
    ///
    /// This function will attempt to parse the user's `credential.helper`
    /// configuration, invoke the necessary processes, and read off what the
    /// username/password should be for a particular url.
    ///
    /// The returned credential type will be a username/password credential if
    /// successful.
    ///
    /// [1]: https://www.kernel.org/pub/software/scm/git/docs/gitcredentials.html
    pub fn credential_helper(config: &Config,
                             url: &str,
                             username: Option<&str>)
                             -> Result<Cred, Error> {
        match CredentialHelper::new(url).config(config).username(username)
                               .execute() {
            Some((username, password)) => {
                Cred::userpass_plaintext(username.as_slice(),
                                         password.as_slice())
            }
            None => Err(Error::from_str("failed to acquire username/password \
                                         from local configuration"))
        }
    }

    /// Check whether a credential object contains username information.
    pub fn has_username(&self) -> bool {
        unsafe { raw::git_cred_has_username(self.raw) == 1 }
    }

    /// Gain access to the underlying raw credential pointer.
    pub fn raw(&self) -> *mut raw::git_cred { self.raw }

    /// Return the type of credentials that this object represents.
    pub fn credtype(&self) -> raw::git_credtype_t {
        unsafe { (*self.raw).credtype }
    }

    /// Unwrap access to the underlying raw pointer, canceling the destructor
    pub unsafe fn unwrap(mut self) -> *mut raw::git_cred {
        mem::replace(&mut self.raw, 0 as *mut raw::git_cred)
    }
}

impl Drop for Cred {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { ((*self.raw).free)(self.raw) }
        }
    }
}

impl CredentialHelper {
    /// Create a new credential helper object which will be used to probe git's
    /// local credential configuration.
    ///
    /// The url specified is the namespace on which this will query credentials.
    /// Invalid urls are currently ignored.
    pub fn new(url: &str) -> CredentialHelper {
        let mut ret = CredentialHelper {
            protocol: None,
            host: None,
            username: None,
            url: url.to_string(),
            commands: Vec::new(),
        };

        // Parse out the (protocol, host) if one is available
        let parsed_url = UrlParser::new().scheme_type_mapper(mapper).parse(url);
        match parsed_url {
            Ok(url) => {
                match url.host() {
                    Some(&url::Host::Domain(ref s)) => ret.host = Some(s.clone()),
                    _ => {}
                }
                ret.protocol = Some(url.scheme)
            }
            Err(..) => {}
        };
        return ret;

        fn mapper(s: &str) -> url::SchemeType {
            match s {
                "git" => url::SchemeType::Relative(9418),
                "ssh" => url::SchemeType::Relative(22),
                s => url::whatwg_scheme_type_mapper(s),
            }
        }
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
        //
        // TODO: implement useHttpPath
        if self.username.is_none() {
            self.config_username(config);
        }
        self.config_helper(config);
        self
    }

    // Configure the queried username from `config`
    fn config_username(&mut self, config: &Config) {
        let key = self.exact_key("username");
        self.username = config.get_str(key.as_slice()).ok().or_else(|| {
            self.url_key("username").and_then(|s| {
                config.get_str(s.as_slice()).ok()
            })
        }).or_else(|| {
            config.get_str("credential.username").ok()
        }).map(|s| s.to_string());
    }

    // Discover all `helper` directives from `config`
    fn config_helper(&mut self, config: &Config) {
        let exact = config.get_str(self.exact_key("helper").as_slice());
        self.add_command(exact.ok());
        match self.url_key("helper") {
            Some(key) => {
                let url = config.get_str(key.as_slice());
                self.add_command(url.ok());
            }
            None => {}
        }
        let global = config.get_str("credential.helper");
        self.add_command(global.ok());
    }

    // Add a `helper` configured command to the list of commands to execute.
    //
    // see https://www.kernel.org/pub/software/scm/git/docs/technical
    //                           /api-credentials.html#_credential_helpers
    fn add_command(&mut self, cmd: Option<&str>) {
        let cmd = match cmd { Some(s) => s, None => return };
        if cmd.starts_with("!") {
            self.commands.push(cmd.slice_from(1).to_string());
        } else if cmd.starts_with("/") || cmd.starts_with("\\") ||
                  cmd.slice_from(1).starts_with(":\\") {
            self.commands.push(format!("\"{}\"", cmd));
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
            _ => None
        }
    }

    /// Execute this helper, attempting to discover a username/password pair.
    ///
    /// All I/O errors are ignored, (to match git behavior), and this function
    /// only succeeds if both a username and a password were found
    pub fn execute(&self) -> Option<(String, String)> {
        let mut username = self.username.clone();
        let mut password = None;
        for cmd in self.commands.iter() {
            let (u, p) = self.execute_cmd(cmd.as_slice(), &username);
            if u.is_some() && username.is_none() {
                username = u;
            }
            if p.is_some() && password.is_none() {
                password = p;
            }
            if username.is_some() && password.is_some() { break }
        }

        match (username, password) {
            (Some(u), Some(p)) => Some((u, p)),
            _ => None,
        }
    }

    // Execute the given `cmd`, providing the appropriate variables on stdin and
    // then afterwards parsing the output into the username/password on stdout.
    fn execute_cmd(&self, cmd: &str, username: &Option<String>)
                   -> (Option<String>, Option<String>) {
        macro_rules! my_try( ($e:expr) => (
            match $e { Ok(e) => e, Err(..) => return (None, None) }
        ) );

        let mut p = my_try!(Command::new("sh").arg("-c")
                                              .arg(format!("{} get", cmd))
                                              .spawn());
        // Ignore write errors as the command may not actually be listening for
        // stdin
        {
            let stdin = p.stdin.as_mut().unwrap();
            match self.protocol {
                Some(ref p) => { let _ = writeln!(stdin, "protocol={}", p); }
                None => {}
            }
            match self.host {
                Some(ref p) => { let _ = writeln!(stdin, "host={}", p); }
                None => {}
            }
            match *username {
                Some(ref p) => { let _ = writeln!(stdin, "username={}", p); }
                None => {}
            }
        }
        let output = my_try!(p.wait_with_output());
        if !output.status.success() { return (None, None) }
        return self.parse_output(output.output)
    }

    // Parse the output of a command into the username/password found
    fn parse_output(&self, output: Vec<u8>) -> (Option<String>, Option<String>) {
        // Parse the output of the command, looking for username/password
        let mut username = None;
        let mut password = None;
        for line in output.as_slice().split(|t| *t == b'\n') {
            let mut parts = line.splitn(1, |t| *t == b'=');
            let key = parts.next().unwrap();
            let value = match parts.next() { Some(s) => s, None => continue };
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
    use std::io::{self, TempDir, File, fs};
    use std::os;

    use {Cred, Config, CredentialHelper, ConfigLevel};

    macro_rules! cfg( ($($k:expr => $v:expr),*) => ({
        let td = TempDir::new("git2-rs").unwrap();
        let mut cfg = Config::new().unwrap();
        cfg.add_file(&td.path().join("cfg"), ConfigLevel::Highest, false).unwrap();
        $(cfg.set_str($k, $v).unwrap();)*
        cfg
    }) );

    #[test]
    fn smoke() {
        Cred::default().unwrap();
    }

    #[test]
    fn credential_helper1() {
        let cfg = cfg! {
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
                                      .config(&cfg)
                                      .execute().unwrap();
        assert_eq!(u.as_slice(), "a");
        assert_eq!(p.as_slice(), "b");
    }

    #[test]
    fn credential_helper2() {
        let cfg = cfg! {};
        assert!(CredentialHelper::new("https://example.com/foo/bar")
                                 .config(&cfg)
                                 .execute().is_none());
    }

    #[test]
    fn credential_helper3() {
        let cfg = cfg! {
            "credential.https://example.com.helper" =>
                    "!f() { echo username=c; }; f",
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
                                      .config(&cfg)
                                      .execute().unwrap();
        assert_eq!(u.as_slice(), "c");
        assert_eq!(p.as_slice(), "b");
    }

    #[test]
    fn credential_helper4() {
        let td = TempDir::new("git2-rs").unwrap();
        let path = td.path().join("script");
        File::create(&path).write_str(r"\
#!/bin/sh
echo username=c
").unwrap();
        fs::chmod(&path, io::USER_EXEC).unwrap();
        let cfg = cfg! {
            "credential.https://example.com.helper" =>
                    path.display().to_string().as_slice(),
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
                                      .config(&cfg)
                                      .execute().unwrap();
        assert_eq!(u.as_slice(), "c");
        assert_eq!(p.as_slice(), "b");
    }

    #[test]
    fn credential_helper5() {
        let td = TempDir::new("git2-rs").unwrap();
        let path = td.path().join("git-credential-script");
        File::create(&path).write_str(r"\
#!/bin/sh
echo username=c
").unwrap();
        fs::chmod(&path, io::USER_EXEC).unwrap();

        let mut paths = os::split_paths(os::getenv("PATH").unwrap().as_slice());
        paths.push(path.dir_path());
        let path = os::join_paths(paths.as_slice()).unwrap();
        os::setenv("PATH", path);

        let cfg = cfg! {
            "credential.https://example.com.helper" => "script",
            "credential.helper" => "!f() { echo username=a; echo password=b; }; f"
        };
        let (u, p) = CredentialHelper::new("https://example.com/foo/bar")
                                      .config(&cfg)
                                      .execute().unwrap();
        assert_eq!(u.as_slice(), "c");
        assert_eq!(p.as_slice(), "b");
    }
}
