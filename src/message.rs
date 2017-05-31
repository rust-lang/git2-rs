use std::ffi::CString;

use libc::{c_char, c_int};

use {raw, Buf, Error, IntoCString};
use util::Binding;

/// Clean up a message, removing extraneous whitespace, and ensure that the
/// message ends with a newline. If `comment_char` is `Some`, also remove comment
/// lines starting with that character.
pub fn message_prettify<T: IntoCString>(message: T, comment_char: Option<u8>)
                                        -> Result<String, Error> {
    _message_prettify(try!(message.into_c_string()), comment_char)
}

fn _message_prettify(message: CString, comment_char: Option<u8>)
                     -> Result<String, Error> {
    let ret = Buf::new();
    unsafe {
        try_call!(raw::git_message_prettify(ret.raw(), message,
                                            comment_char.is_some() as c_int,
                                            comment_char.unwrap_or(0) as c_char));
    }
    Ok(ret.as_str().unwrap().to_string())
}

/// The default comment character for `message_prettify` ('#')
pub const DEFAULT_COMMENT_CHAR: Option<u8> = Some(b'#');

#[cfg(test)]
mod tests {
    use {message_prettify, DEFAULT_COMMENT_CHAR};

    #[test]
    fn prettify() {
        // This does not attempt to duplicate the extensive tests for
        // git_message_prettify in libgit2, just a few representative values to
        // make sure the interface works as expected.
        assert_eq!(message_prettify("1\n\n\n2", None).unwrap(),
                   "1\n\n2\n");
        assert_eq!(message_prettify("1\n\n\n2\n\n\n3", None).unwrap(),
                   "1\n\n2\n\n3\n");
        assert_eq!(message_prettify("1\n# comment\n# more", None).unwrap(),
                   "1\n# comment\n# more\n");
        assert_eq!(message_prettify("1\n# comment\n# more",
                                    DEFAULT_COMMENT_CHAR).unwrap(),
                   "1\n");
        assert_eq!(message_prettify("1\n; comment\n; more",
                                    Some(';' as u8)).unwrap(),
                   "1\n");
    }
}
