use crate::raw;
use std::convert::TryFrom;
use std::ptr;
use std::str::{self, Utf8Error};

macro_rules! define_inspector {
    ($enum_name:ident, $enum_doc:literal, $fn_doc:literal, $s:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[doc = $enum_doc]
        pub enum $enum_name<'string> {
            /// The attribute is set to true.
            True,
            /// The attribute is unset (set to false).
            False,
            /// The attribute is set to a string.
            String(&'string $s),
            /// The attribute is not specified.
            Unspecified,
        }

        impl<'string> $enum_name<'string> {
            #[doc = $fn_doc]
            pub fn new(attr_value: Option<&'string $s>) -> Self {
                match unsafe {
                    raw::git_attr_value(attr_value.map_or(ptr::null(), |v| v.as_ptr().cast()))
                } {
                    raw::GIT_ATTR_VALUE_TRUE => $enum_name::True,
                    raw::GIT_ATTR_VALUE_FALSE => $enum_name::False,
                    raw::GIT_ATTR_VALUE_STRING => $enum_name::String(attr_value.unwrap()),
                    raw::GIT_ATTR_VALUE_UNSPECIFIED => $enum_name::Unspecified,
                    _ => unreachable!(),
                }
            }
        }
    };
}

define_inspector!(
    AttrValue,
    "All possible states of an attribute, using [`prim@str`] to represent the string.\n\n\
     This enum is used to interpret the value returned by \
     [`Repository::get_attr`](crate::Repository::get_attr).",
    "Returns the state of an attribute by inspecting its [value](crate::Repository::get_attr) \
     by a [string](prim@str).",
    str
);

/// Converts [`AttrValueBytes`] to [`AttrValue`]. If the attribute is [set to a string](`AttrValueBytes::String`),
/// this implementation will use [`str::from_utf8`] to perform the conversion.
impl<'string> TryFrom<AttrValueBytes<'string>> for AttrValue<'string> {
    type Error = Utf8Error;

    fn try_from(value: AttrValueBytes<'string>) -> Result<Self, Self::Error> {
        match value {
            AttrValueBytes::True => Ok(Self::True),
            AttrValueBytes::False => Ok(Self::False),
            AttrValueBytes::String(s) => Ok(Self::String(str::from_utf8(s)?)),
            AttrValueBytes::Unspecified => Ok(Self::Unspecified),
        }
    }
}

define_inspector!(
    AttrValueBytes,
    "All possible states of an attribute, using a [byte](u8) [slice] to represent the string.\n\n\
     This enum is used to interpret the value returned by \
     [`Repository::get_attr_bytes`](crate::Repository::get_attr_bytes).",
    "Returns the state of an attribute by inspecting its [value](crate::Repository::get_attr_bytes) \
     by a [byte](u8) [slice].",
    [u8]
);

/// Converts [`AttrValue`] to [`AttrValueBytes`]. This implementation will convert the
/// [string slice](prim@str) to a [byte](u8) [slice] when the attribute is
/// [set to a string](`AttrValue::String`).
impl<'string> From<AttrValue<'string>> for AttrValueBytes<'string> {
    fn from(value: AttrValue<'string>) -> Self {
        match value {
            AttrValue::True => Self::True,
            AttrValue::False => Self::False,
            AttrValue::String(s) => Self::String(s.as_ref()),
            AttrValue::Unspecified => Self::Unspecified,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AttrValue, AttrValueBytes};
    use std::convert::{TryFrom, TryInto};
    use std::ffi::CStr;
    use std::os::raw::c_char;

    extern "C" {
        // libgit2 defines them as mutable, so they are also declared mutable here.
        // However, libgit2 never mutates them, thus it's always safe to access them in Rust.
        static mut git_attr__true: *const c_char;
        static mut git_attr__false: *const c_char;
        static mut git_attr__unset: *const c_char;
    }

    macro_rules! test_enum {
        ($name:ident) => {
            let attr_true = unsafe { CStr::from_ptr(git_attr__true) }.to_str().unwrap();
            let attr_false = unsafe { CStr::from_ptr(git_attr__false) }.to_str().unwrap();
            let attr_unset = unsafe { CStr::from_ptr(git_attr__unset) }.to_str().unwrap();
            assert_eq!(
                $name::new(Some(attr_true.to_owned().as_ref())),
                $name::String(attr_true.as_ref())
            );
            assert_eq!(
                $name::new(Some(attr_false.to_owned().as_ref())),
                $name::String(attr_false.as_ref())
            );
            assert_eq!(
                $name::new(Some(attr_unset.to_owned().as_ref())),
                $name::String(attr_unset.as_ref())
            );
            assert_eq!(
                $name::new(Some("foo".as_ref())),
                $name::String("foo".as_ref())
            );
            assert_eq!(
                $name::new(Some("bar".as_ref())),
                $name::String("bar".as_ref())
            );
            assert_eq!($name::new(Some(attr_true.as_ref())), $name::True);
            assert_eq!($name::new(Some(attr_false.as_ref())), $name::False);
            assert_eq!($name::new(Some(attr_unset.as_ref())), $name::Unspecified);
            assert_eq!($name::new(None), $name::Unspecified);
        };
    }

    #[test]
    fn attr_value() {
        test_enum!(AttrValue);
    }

    #[test]
    fn attr_value_bytes() {
        test_enum!(AttrValueBytes);
    }

    #[test]
    fn attr_value_from_attr_value_bytes() {
        assert_eq!(AttrValue::True, AttrValueBytes::True.try_into().unwrap());
        assert_eq!(AttrValue::False, AttrValueBytes::False.try_into().unwrap());
        assert_eq!(
            AttrValue::String("foo"),
            AttrValueBytes::String(b"foo").try_into().unwrap()
        );
        assert_eq!(
            AttrValue::try_from(AttrValueBytes::String(b"bar\xff"))
                .unwrap_err()
                .valid_up_to(),
            3
        );
        assert_eq!(
            AttrValue::Unspecified,
            AttrValueBytes::Unspecified.try_into().unwrap()
        );
    }

    #[test]
    fn attr_value_bytes_from_attr_value() {
        assert_eq!(AttrValueBytes::True, AttrValue::True.into());
        assert_eq!(AttrValueBytes::False, AttrValue::False.into());
        assert_eq!(
            AttrValueBytes::String(b"foo"),
            AttrValue::String("foo").into()
        );
        assert_eq!(AttrValueBytes::Unspecified, AttrValue::Unspecified.into());
    }
}
