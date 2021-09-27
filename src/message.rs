use std::ffi::CString;
use std::ffi::CStr;
use std::ptr;
use std::marker;

use libc::{c_char, c_int};

use crate::util::Binding;
use crate::{raw, Buf, Error, IntoCString};

/// Clean up a message, removing extraneous whitespace, and ensure that the
/// message ends with a newline. If `comment_char` is `Some`, also remove comment
/// lines starting with that character.
pub fn message_prettify<T: IntoCString>(
    message: T,
    comment_char: Option<u8>,
) -> Result<String, Error> {
    _message_prettify(message.into_c_string()?, comment_char)
}

fn _message_prettify(message: CString, comment_char: Option<u8>) -> Result<String, Error> {
    let ret = Buf::new();
    unsafe {
        try_call!(raw::git_message_prettify(
            ret.raw(),
            message,
            comment_char.is_some() as c_int,
            comment_char.unwrap_or(0) as c_char
        ));
    }
    Ok(ret.as_str().unwrap().to_string())
}

/// Collection of trailer key–value pairs.
///
/// Use `iter()` to get access to the values.
pub struct MessageTrailers<'pair> {
    raw: raw::git_message_trailer_array,
    _marker: marker::PhantomData<&'pair c_char>,
}

impl<'pair> MessageTrailers<'pair> {
    fn new() -> MessageTrailers<'pair> {
        crate::init();
        unsafe {
            Binding::from_raw(&mut raw::git_message_trailer_array {
                trailers: ptr::null_mut(),
                count: 0,
                _trailer_block: ptr::null_mut(),
            } as *mut _)
        }
    }
    /// Create a borrowed iterator.
    pub fn iter(&'pair self) -> MessageTrailersIterator<'pair> {
        MessageTrailersIterator {
            trailers: self,
            counter: 0,
        }
    }
    /// The number of trailer key–value pairs.
    pub fn len(&self) -> usize {
        self.raw.count
    }
}

impl<'pair> Drop for MessageTrailers<'pair> {
    fn drop(&mut self) {
        unsafe {
            raw::git_message_trailer_array_free(&mut self.raw);
        }
    }
}

impl<'pair> Binding for MessageTrailers<'pair> {
    type Raw = *mut raw::git_message_trailer_array;
    unsafe fn from_raw(
        raw: *mut raw::git_message_trailer_array
    ) -> MessageTrailers<'pair> {
        MessageTrailers {
            raw: *raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_message_trailer_array {
        &self.raw as *const _ as *mut _
    }
}

struct Trailer<'pair> {
    key: *const c_char,
    value: *const c_char,
    _marker: marker::PhantomData<&'pair c_char>,
}

impl<'pair> Trailer<'pair> {
    fn to_str_tuple(self) -> (&'pair str, &'pair str) {
        unsafe {
            let key = CStr::from_ptr(self.key).to_str().unwrap();
            let value = CStr::from_ptr(self.value).to_str().unwrap();
            (key, value)
        }
    }
}

/// A borrowed iterator.
pub struct MessageTrailersIterator<'pair> {
    trailers: &'pair MessageTrailers<'pair>,
    counter: usize,
}

impl<'pair> Iterator for MessageTrailersIterator<'pair> {
    type Item = (&'pair str, &'pair str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter == self.trailers.raw.count {
            None
        } else {
            unsafe {
                let addr = self.trailers.raw.trailers.wrapping_add(self.counter);
                self.counter += 1;
                Some(Trailer {
                    key: (*addr).key,
                    value: (*addr).value,
                    _marker: marker::PhantomData,
                }.to_str_tuple())
            }
        }
    }
}

/// Get the trailers for the given message.
pub fn message_trailers<'pair, S: IntoCString>(
    message: S
) -> Result<MessageTrailers<'pair>, Error> {
    _message_trailers(message.into_c_string()?)
}

fn _message_trailers<'pair>(
    message: CString
) -> Result<MessageTrailers<'pair>, Error> {
    let ret = MessageTrailers::new();
    unsafe {
        try_call!(raw::git_message_trailers(
            ret.raw(),
            message
        ));
    }
    Ok(ret)
}

/// The default comment character for `message_prettify` ('#')
pub const DEFAULT_COMMENT_CHAR: Option<u8> = Some(b'#');

#[cfg(test)]
mod tests {

    #[test]
    fn prettify() {
        use crate::{message_prettify, DEFAULT_COMMENT_CHAR};

        // This does not attempt to duplicate the extensive tests for
        // git_message_prettify in libgit2, just a few representative values to
        // make sure the interface works as expected.
        assert_eq!(message_prettify("1\n\n\n2", None).unwrap(), "1\n\n2\n");
        assert_eq!(
            message_prettify("1\n\n\n2\n\n\n3", None).unwrap(),
            "1\n\n2\n\n3\n"
        );
        assert_eq!(
            message_prettify("1\n# comment\n# more", None).unwrap(),
            "1\n# comment\n# more\n"
        );
        assert_eq!(
            message_prettify("1\n# comment\n# more", DEFAULT_COMMENT_CHAR).unwrap(),
            "1\n"
        );
        assert_eq!(
            message_prettify("1\n; comment\n; more", Some(';' as u8)).unwrap(),
            "1\n"
        );
    }

    #[test]
    fn trailers() {
        use std::collections::HashMap;
        use crate::{message_trailers, MessageTrailers};

        // no trailers
        let message1 = "
WHAT ARE WE HERE FOR

What are we here for?

Just to be eaten?
";
        let expected: HashMap<&str, &str> = HashMap::new();
        assert_eq!(expected, to_map(&message_trailers(message1).unwrap()));

        // standard PSA
        let message2 = "
Attention all

We are out of tomatoes.

Spoken-by: Major Turnips
Transcribed-by: Seargant Persimmons
Signed-off-by: Colonel Kale
";
        let expected: HashMap<&str, &str> = vec![
            ("Spoken-by", "Major Turnips"),
            ("Transcribed-by", "Seargant Persimmons"),
            ("Signed-off-by", "Colonel Kale")
        ].into_iter().collect();
        assert_eq!(expected, to_map(&message_trailers(message2).unwrap()));

        // ignore everything after `---`
        let message3 = "
The fate of Seargant Green-Peppers

Seargant Green-Peppers was killed by Caterpillar Battalion 44.

Signed-off-by: Colonel Kale
---
I never liked that guy, anyway.

Opined-by: Corporal Garlic
";
        let expected: HashMap<&str, &str> = vec![
            ("Signed-off-by", "Colonel Kale")
        ].into_iter().collect();
        assert_eq!(expected, to_map(&message_trailers(message3).unwrap()));

        fn to_map<'pair>(
            trailers: &'pair MessageTrailers<'pair>
        ) -> HashMap<&str, &str> {
            let mut map = HashMap::with_capacity(trailers.len());
            for (key, value) in trailers.iter() {
                map.insert(key, value);
            }
            map
        }
    }
}
