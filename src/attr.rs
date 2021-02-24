use crate::raw;
use std::ptr;
use std::str;

/// All possible states of an attribute.
///
/// This enum is used to interpret the value returned by
/// [`Repository::get_attr`](crate::Repository::get_attr) and
/// [`Repository::get_attr_bytes`](crate::Repository::get_attr_bytes).
#[derive(Debug, Clone, Copy, Eq)]
pub enum AttrValue<'string> {
    /// The attribute is set to true.
    True,
    /// The attribute is unset (set to false).
    False,
    /// The attribute is set to a [valid UTF-8 string](prim@str).
    String(&'string str),
    /// The attribute is set to a non-UTF-8 string.
    Bytes(&'string [u8]),
    /// The attribute is not specified.
    Unspecified,
}

impl<'string> AttrValue<'string> {
    /// Returns the state of an attribute by inspecting its [value](crate::Repository::get_attr)
    /// by a [string](prim@str).
    ///
    /// As [`str`](prim@str) is guaranteed to contain only valid UTF-8, this function never returns
    /// [`AttrValue::Bytes`].
    pub fn from_string(value: Option<&'string str>) -> Self {
        match unsafe { raw::git_attr_value(value.map_or(ptr::null(), |v| v.as_ptr().cast())) } {
            raw::GIT_ATTR_VALUE_TRUE => Self::True,
            raw::GIT_ATTR_VALUE_FALSE => Self::False,
            raw::GIT_ATTR_VALUE_STRING => Self::String(value.unwrap()),
            raw::GIT_ATTR_VALUE_UNSPECIFIED => Self::Unspecified,
            _ => unreachable!(),
        }
    }

    /// Returns the state of an attribute by inspecting its [value](crate::Repository::get_attr_bytes)
    /// by a [byte](u8) [slice].
    pub fn from_bytes(value: Option<&'string [u8]>) -> Self {
        match unsafe { raw::git_attr_value(value.map_or(ptr::null(), |v| v.as_ptr().cast())) } {
            raw::GIT_ATTR_VALUE_TRUE => Self::True,
            raw::GIT_ATTR_VALUE_FALSE => Self::False,
            raw::GIT_ATTR_VALUE_STRING => {
                let value = value.unwrap();
                if let Ok(string) = str::from_utf8(value) {
                    Self::String(string)
                } else {
                    Self::Bytes(value)
                }
            }
            raw::GIT_ATTR_VALUE_UNSPECIFIED => Self::Unspecified,
            _ => unreachable!(),
        }
    }
}

/// Compare two [`AttrValue`]s.
///
/// Note that this implementation does not differentiate [`AttrValue::String`] and
/// [`AttrValue::Bytes`].
impl PartialEq for AttrValue<'_> {
    fn eq(&self, other: &AttrValue<'_>) -> bool {
        match (self, other) {
            (Self::True, AttrValue::True)
            | (Self::False, AttrValue::False)
            | (Self::Unspecified, AttrValue::Unspecified) => true,
            (AttrValue::String(string), AttrValue::Bytes(bytes))
            | (Self::Bytes(bytes), AttrValue::String(string)) => string.as_bytes() == *bytes,
            (Self::String(left), AttrValue::String(right)) => left == right,
            (Self::Bytes(left), AttrValue::Bytes(right)) => left == right,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AttrValue;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    extern "C" {
        // libgit2 defines them as mutable, so they are also declared mutable here.
        // However, libgit2 never mutates them, thus it's always safe to access them in Rust.
        static mut git_attr__true: *const c_char;
        static mut git_attr__false: *const c_char;
        static mut git_attr__unset: *const c_char;
    }

    macro_rules! test_attr_value_from {
        ($function:ident, $variant:ident) => {
            let attr_true = unsafe { CStr::from_ptr(git_attr__true) }.to_str().unwrap();
            let attr_false = unsafe { CStr::from_ptr(git_attr__false) }.to_str().unwrap();
            let attr_unset = unsafe { CStr::from_ptr(git_attr__unset) }.to_str().unwrap();
            assert_eq!(
                AttrValue::$function(Some(attr_true.to_owned().as_ref())),
                AttrValue::$variant(attr_true.as_ref())
            );
            assert_eq!(
                AttrValue::$function(Some(attr_false.to_owned().as_ref())),
                AttrValue::$variant(attr_false.as_ref())
            );
            assert_eq!(
                AttrValue::$function(Some(attr_unset.to_owned().as_ref())),
                AttrValue::$variant(attr_unset.as_ref())
            );
            assert_eq!(
                AttrValue::$function(Some("foo".as_ref())),
                AttrValue::$variant("foo".as_ref())
            );
            assert_eq!(
                AttrValue::$function(Some("bar".as_ref())),
                AttrValue::$variant("bar".as_ref())
            );
            assert_eq!(
                AttrValue::$function(Some(attr_true.as_ref())),
                AttrValue::True
            );
            assert_eq!(
                AttrValue::$function(Some(attr_false.as_ref())),
                AttrValue::False
            );
            assert_eq!(
                AttrValue::$function(Some(attr_unset.as_ref())),
                AttrValue::Unspecified
            );
            assert_eq!(AttrValue::$function(None), AttrValue::Unspecified);
        };
    }

    #[test]
    fn attr_value_from_string() {
        test_attr_value_from!(from_string, String);
    }

    #[test]
    fn attr_value_from_bytes() {
        test_attr_value_from!(from_bytes, Bytes);
    }

    #[test]
    fn attr_value_partial_eq() {
        assert_eq!(AttrValue::True, AttrValue::True);
        assert_eq!(AttrValue::False, AttrValue::False);
        assert_eq!(AttrValue::String("foo"), AttrValue::String("foo"));
        assert_eq!(AttrValue::Bytes(b"foo"), AttrValue::Bytes(b"foo"));
        assert_eq!(AttrValue::String("bar"), AttrValue::Bytes(b"bar"));
        assert_eq!(AttrValue::Bytes(b"bar"), AttrValue::String("bar"));
        assert_eq!(AttrValue::Unspecified, AttrValue::Unspecified);
        assert_ne!(AttrValue::True, AttrValue::False);
        assert_ne!(AttrValue::False, AttrValue::Unspecified);
        assert_ne!(AttrValue::Unspecified, AttrValue::True);
        assert_ne!(AttrValue::True, AttrValue::String("true"));
        assert_ne!(AttrValue::Unspecified, AttrValue::Bytes(b"unspecified"));
        assert_ne!(AttrValue::Bytes(b"false"), AttrValue::False);
        assert_ne!(AttrValue::String("unspecified"), AttrValue::Unspecified);
        assert_ne!(AttrValue::String("foo"), AttrValue::String("bar"));
        assert_ne!(AttrValue::Bytes(b"foo"), AttrValue::Bytes(b"bar"));
        assert_ne!(AttrValue::String("foo"), AttrValue::Bytes(b"bar"));
        assert_ne!(AttrValue::Bytes(b"foo"), AttrValue::String("bar"));
    }
}
