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
    /// The attribute is set to a string that might not be [valid UTF-8](prim@str).
    Bytes(&'string [u8]),
    /// The attribute is not specified.
    Unspecified,
}

macro_rules! from_value {
    ($value:expr => $string:expr) => {
        match unsafe { raw::git_attr_value($value.map_or(ptr::null(), |v| v.as_ptr().cast())) } {
            raw::GIT_ATTR_VALUE_TRUE => Self::True,
            raw::GIT_ATTR_VALUE_FALSE => Self::False,
            raw::GIT_ATTR_VALUE_STRING => $string,
            raw::GIT_ATTR_VALUE_UNSPECIFIED => Self::Unspecified,
            _ => unreachable!(),
        }
    };
}

impl<'string> AttrValue<'string> {
    /// Returns the state of an attribute by inspecting its [value](crate::Repository::get_attr)
    /// by a [string](prim@str).
    ///
    /// This function always returns [`AttrValue::String`] and never returns [`AttrValue::Bytes`]
    /// when the attribute is set to a string.
    pub fn from_string(value: Option<&'string str>) -> Self {
        from_value!(value => Self::String(value.unwrap()))
    }

    /// Returns the state of an attribute by inspecting its [value](crate::Repository::get_attr_bytes)
    /// by a [byte](u8) [slice].
    ///
    /// This function will perform UTF-8 validation when the attribute is set to a string, returns
    /// [`AttrValue::String`] if it's valid UTF-8 and [`AttrValue::Bytes`] otherwise.
    pub fn from_bytes(value: Option<&'string [u8]>) -> Self {
        let mut value = Self::always_bytes(value);
        if let Self::Bytes(bytes) = value {
            if let Ok(string) = str::from_utf8(bytes) {
                value = Self::String(string);
            }
        }
        value
    }

    /// Returns the state of an attribute just like [`AttrValue::from_bytes`], but skips UTF-8
    /// validation and always returns [`AttrValue::Bytes`] when it's set to a string.
    pub fn always_bytes(value: Option<&'string [u8]>) -> Self {
        from_value!(value => Self::Bytes(value.unwrap()))
    }
}

/// Compare two [`AttrValue`]s.
///
/// Note that this implementation does not differentiate between [`AttrValue::String`] and
/// [`AttrValue::Bytes`].
impl PartialEq for AttrValue<'_> {
    fn eq(&self, other: &AttrValue<'_>) -> bool {
        match (self, other) {
            (Self::True, AttrValue::True)
            | (Self::False, AttrValue::False)
            | (Self::Unspecified, AttrValue::Unspecified) => true,
            (AttrValue::String(string), AttrValue::Bytes(bytes))
            | (AttrValue::Bytes(bytes), AttrValue::String(string)) => string.as_bytes() == *bytes,
            (AttrValue::String(left), AttrValue::String(right)) => left == right,
            (AttrValue::Bytes(left), AttrValue::Bytes(right)) => left == right,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AttrValue;

    macro_rules! test_attr_value {
        ($function:ident, $variant:ident) => {
            const ATTR_TRUE: &str = "[internal]__TRUE__";
            const ATTR_FALSE: &str = "[internal]__FALSE__";
            const ATTR_UNSET: &str = "[internal]__UNSET__";
            let as_bytes = AsRef::<[u8]>::as_ref;
            // Use `matches!` here since the `PartialEq` implementation does not differentiate
            // between `String` and `Bytes`.
            assert!(matches!(
                AttrValue::$function(Some(ATTR_TRUE.as_ref())),
                AttrValue::$variant(s) if as_bytes(s) == ATTR_TRUE.as_bytes()
            ));
            assert!(matches!(
                AttrValue::$function(Some(ATTR_FALSE.as_ref())),
                AttrValue::$variant(s) if as_bytes(s) == ATTR_FALSE.as_bytes()
            ));
            assert!(matches!(
                AttrValue::$function(Some(ATTR_UNSET.as_ref())),
                AttrValue::$variant(s) if as_bytes(s) == ATTR_UNSET.as_bytes()
            ));
            assert!(matches!(
                AttrValue::$function(Some("foo".as_ref())),
                AttrValue::$variant(s) if as_bytes(s) == b"foo"
            ));
            assert!(matches!(
                AttrValue::$function(Some("bar".as_ref())),
                AttrValue::$variant(s) if as_bytes(s) == b"bar"
            ));
            assert_eq!(AttrValue::$function(None), AttrValue::Unspecified);
        };
    }

    #[test]
    fn attr_value_from_string() {
        test_attr_value!(from_string, String);
    }

    #[test]
    fn attr_value_from_bytes() {
        test_attr_value!(from_bytes, String);
        assert!(matches!(
            AttrValue::from_bytes(Some(&[0xff])),
            AttrValue::Bytes(&[0xff])
        ));
        assert!(matches!(
            AttrValue::from_bytes(Some(b"\xffoobar")),
            AttrValue::Bytes(b"\xffoobar")
        ));
    }

    #[test]
    fn attr_value_always_bytes() {
        test_attr_value!(always_bytes, Bytes);
        assert!(matches!(
            AttrValue::always_bytes(Some(&[0xff; 2])),
            AttrValue::Bytes(&[0xff, 0xff])
        ));
        assert!(matches!(
            AttrValue::always_bytes(Some(b"\xffoo")),
            AttrValue::Bytes(b"\xffoo")
        ));
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
