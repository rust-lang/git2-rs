use core::ops::Range;
use std::ffi::CStr;
use std::ffi::CString;
use std::iter::FusedIterator;
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

/// The default comment character for `message_prettify` ('#')
pub const DEFAULT_COMMENT_CHAR: Option<u8> = Some(b'#');

/// Get the trailers for the given message.
///
/// Use this function when you are dealing with a UTF-8-encoded message.
pub fn message_trailers_strs(message: &str) -> Result<MessageTrailersStrs, Error> {
    _message_trailers(message.into_c_string()?).map(|res| MessageTrailersStrs(res))
}

/// Get the trailers for the given message.
///
/// Use this function when the message might not be UTF-8-encoded,
/// or if you want to handle the returned trailer key–value pairs
/// as bytes.
pub fn message_trailers_bytes<S: IntoCString>(message: S) -> Result<MessageTrailersBytes, Error> {
    _message_trailers(message.into_c_string()?).map(|res| MessageTrailersBytes(res))
}

fn _message_trailers(message: CString) -> Result<MessageTrailers, Error> {
    let ret = MessageTrailers::new();
    unsafe {
        try_call!(raw::git_message_trailers(ret.raw(), message));
    }
    Ok(ret)
}

/// Collection of UTF-8-encoded trailers.
///
/// Use `iter()` to get access to the values.
pub struct MessageTrailersStrs(MessageTrailers);

impl MessageTrailersStrs {
    /// Create a borrowed iterator.
    pub fn iter(&self) -> MessageTrailersStrsIterator<'_> {
        MessageTrailersStrsIterator(self.0.iter())
    }
    /// The number of trailer key–value pairs.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Convert to the “bytes” variant.
    pub fn to_bytes(self) -> MessageTrailersBytes {
        MessageTrailersBytes(self.0)
    }
}

/// Collection of unencoded (bytes) trailers.
///
/// Use `iter()` to get access to the values.
pub struct MessageTrailersBytes(MessageTrailers);

impl MessageTrailersBytes {
    /// Create a borrowed iterator.
    pub fn iter(&self) -> MessageTrailersBytesIterator<'_> {
        MessageTrailersBytesIterator(self.0.iter())
    }
    /// The number of trailer key–value pairs.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

struct MessageTrailers {
    raw: raw::git_message_trailer_array,
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
    fn iter(&self) -> MessageTrailersIterator<'_> {
        MessageTrailersIterator {
            trailers: self,
            range: Range {
                start: 0,
                end: self.raw.count,
            },
        }
    }
    fn len(&self) -> usize {
        self.raw.count
    }
}

impl Drop for MessageTrailers {
    fn drop(&mut self) {
        unsafe {
            raw::git_message_trailer_array_free(&mut self.raw);
        }
    }
}

impl Binding for MessageTrailers {
    type Raw = *mut raw::git_message_trailer_array;
    unsafe fn from_raw(raw: *mut raw::git_message_trailer_array) -> MessageTrailers {
        MessageTrailers { raw: *raw }
    }
    fn raw(&self) -> *mut raw::git_message_trailer_array {
        &self.raw as *const _ as *mut _
    }
}

struct MessageTrailersIterator<'a> {
    trailers: &'a MessageTrailers,
    range: Range<usize>,
}

fn to_raw_tuple(trailers: &MessageTrailers, index: usize) -> (*const c_char, *const c_char) {
    unsafe {
        let addr = trailers.raw.trailers.wrapping_add(index);
        ((*addr).key, (*addr).value)
    }
}

/// Borrowed iterator over the UTF-8-encoded trailers.
pub struct MessageTrailersStrsIterator<'a>(MessageTrailersIterator<'a>);

impl<'pair> Iterator for MessageTrailersStrsIterator<'pair> {
    type Item = (&'pair str, &'pair str);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .range
            .next()
            .map(|index| to_str_tuple(&self.0.trailers, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.range.size_hint()
    }
}

impl FusedIterator for MessageTrailersStrsIterator<'_> {}

impl ExactSizeIterator for MessageTrailersStrsIterator<'_> {
    fn len(&self) -> usize {
        self.0.range.len()
    }
}

impl DoubleEndedIterator for MessageTrailersStrsIterator<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0
            .range
            .next_back()
            .map(|index| to_str_tuple(&self.0.trailers, index))
    }
}

fn to_str_tuple(trailers: &MessageTrailers, index: usize) -> (&str, &str) {
    unsafe {
        let (rkey, rvalue) = to_raw_tuple(&trailers, index);
        let key = CStr::from_ptr(rkey).to_str().unwrap();
        let value = CStr::from_ptr(rvalue).to_str().unwrap();
        (key, value)
    }
}

/// Borrowed iterator over the raw (bytes) trailers.
pub struct MessageTrailersBytesIterator<'a>(MessageTrailersIterator<'a>);

impl<'pair> Iterator for MessageTrailersBytesIterator<'pair> {
    type Item = (&'pair [u8], &'pair [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .range
            .next()
            .map(|index| to_bytes_tuple(&self.0.trailers, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.range.size_hint()
    }
}

impl FusedIterator for MessageTrailersBytesIterator<'_> {}

impl ExactSizeIterator for MessageTrailersBytesIterator<'_> {
    fn len(&self) -> usize {
        self.0.range.len()
    }
}

impl DoubleEndedIterator for MessageTrailersBytesIterator<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0
            .range
            .next_back()
            .map(|index| to_bytes_tuple(&self.0.trailers, index))
    }
}

fn to_bytes_tuple(trailers: &MessageTrailers, index: usize) -> (&[u8], &[u8]) {
    unsafe {
        let (rkey, rvalue) = to_raw_tuple(&trailers, index);
        let key = CStr::from_ptr(rkey).to_bytes();
        let value = CStr::from_ptr(rvalue).to_bytes();
        (key, value)
    }
}

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
        use crate::{message_trailers_bytes, message_trailers_strs, MessageTrailersStrs};
        use std::collections::HashMap;

        // no trailers
        let message1 = "
WHAT ARE WE HERE FOR

What are we here for?

Just to be eaten?
";
        let expected: HashMap<&str, &str> = HashMap::new();
        assert_eq!(expected, to_map(&message_trailers_strs(message1).unwrap()));

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
        assert_eq!(expected, to_map(&message_trailers_strs(message2).unwrap()));

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
        assert_eq!(expected, to_map(&message_trailers_strs(message3).unwrap()));

        // Raw bytes message; not valid UTF-8
        // Source: https://stackoverflow.com/a/3886015/1725151
        let message4 = b"
Be honest guys

Am I a malformed brussels sprout?

Signed-off-by: Lieutenant \xe2\x28\xa1prout
";

        let trailer = message_trailers_bytes(&message4[..]).unwrap();
        let expected = (&b"Signed-off-by"[..], &b"Lieutenant \xe2\x28\xa1prout"[..]);
        let actual = trailer.iter().next().unwrap();
        assert_eq!(expected, actual);

        fn to_map(trailers: &MessageTrailersStrs) -> HashMap<&str, &str> {
            let mut map = HashMap::with_capacity(trailers.len());
            for (key, value) in trailers.iter() {
                map.insert(key, value);
            }
            map
        }
    }
}
