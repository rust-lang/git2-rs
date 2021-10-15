use core::ops::Range;
use std::ffi::CStr;
use std::ffi::CString;
use std::marker;
use std::ptr;

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
pub struct MessageTrailers {
    raw: raw::git_message_trailer_array,
    _marker: marker::PhantomData<c_char>,
}

impl MessageTrailers {
    fn new() -> MessageTrailers {
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
    pub fn iter(&self) -> MessageTrailersIterator<'_> {
        MessageTrailersIterator {
            trailers: self,
            range: Range {
                start: 0,
                end: self.raw.count,
            },
        }
    }
    /// The number of trailer key–value pairs.
    pub fn len(&self) -> usize {
        self.raw.count
    }
}

impl<'pair> Drop for MessageTrailers {
    fn drop(&mut self) {
        unsafe {
            raw::git_message_trailer_array_free(&mut self.raw);
        }
    }
}

impl Binding for MessageTrailers {
    type Raw = *mut raw::git_message_trailer_array;
    unsafe fn from_raw(raw: *mut raw::git_message_trailer_array) -> MessageTrailers {
        MessageTrailers {
            raw: *raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_message_trailer_array {
        &self.raw as *const _ as *mut _
    }
}

/// A borrowed iterator.
pub struct MessageTrailersIterator<'a> {
    trailers: &'a MessageTrailers,
    range: Range<usize>,
}

impl<'pair> Iterator for MessageTrailersIterator<'pair> {
    type Item = (&'pair str, &'pair str);

    fn next(&mut self) -> Option<Self::Item> {
        self.range
            .next()
            .map(|index| to_str_tuple(&self.trailers, index, marker::PhantomData))
    }
}

#[inline(always)]
fn to_str_tuple(
    trailers: &MessageTrailers,
    index: usize,
    _marker: marker::PhantomData<c_char>,
) -> (&str, &str) {
    unsafe {
        let addr = trailers.raw.trailers.wrapping_add(index);
        let key = CStr::from_ptr((*addr).key).to_str().unwrap();
        let value = CStr::from_ptr((*addr).value).to_str().unwrap();
        (key, value)
    }
}

impl ExactSizeIterator for MessageTrailersIterator<'_> {
    fn len(&self) -> usize {
        self.range.end
    }
}

impl DoubleEndedIterator for MessageTrailersIterator<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range
            .next_back()
            .map(|index| to_str_tuple(&self.trailers, index, marker::PhantomData))
    }
}

/// Get the trailers for the given message.
pub fn message_trailers<'pair, S: IntoCString>(message: S) -> Result<MessageTrailers, Error> {
    _message_trailers(message.into_c_string()?)
}

fn _message_trailers<'pair>(message: CString) -> Result<MessageTrailers, Error> {
    let ret = MessageTrailers::new();
    unsafe {
        try_call!(raw::git_message_trailers(ret.raw(), message));
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
        use crate::{message_trailers, MessageTrailers};
        use std::collections::HashMap;

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
            ("Signed-off-by", "Colonel Kale"),
        ]
        .into_iter()
        .collect();
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
        let expected: HashMap<&str, &str> = vec![("Signed-off-by", "Colonel Kale")]
            .into_iter()
            .collect();
        assert_eq!(expected, to_map(&message_trailers(message3).unwrap()));

        fn to_map(trailers: &MessageTrailers) -> HashMap<&str, &str> {
            let mut map = HashMap::with_capacity(trailers.len());
            for (key, value) in trailers.iter() {
                map.insert(key, value);
            }
            map
        }
    }
}
