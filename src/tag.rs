use std::ffi::CString;
use std::marker;
use std::mem;
use std::ptr;
use std::str;

use crate::util::Binding;
use crate::{call, raw, signature, Error, Object, ObjectType, Oid, Signature};

/// A structure to represent a git [tag][1]
///
/// [1]: http://git-scm.com/book/en/Git-Basics-Tagging
pub struct Tag<'repo> {
    raw: *mut raw::git_tag,
    _marker: marker::PhantomData<Object<'repo>>,
}

impl<'repo> Tag<'repo> {
    /// Determine whether a tag name is valid, meaning that (when prefixed with refs/tags/) that
    /// it is a valid reference name, and that any additional tag name restrictions are imposed
    /// (eg, it cannot start with a -).
    pub fn is_valid_name(tag_name: &str) -> bool {
        crate::init();
        let tag_name = CString::new(tag_name).unwrap();
        let mut valid: libc::c_int = 0;
        unsafe {
            call::c_try(raw::git_tag_name_is_valid(&mut valid, tag_name.as_ptr())).unwrap();
        }
        valid == 1
    }

    /// Get the id (SHA1) of a repository tag
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_tag_id(&*self.raw)) }
    }

    /// Get the message of a tag
    ///
    /// Returns None if there is no message or if it is not valid utf8
    pub fn message(&self) -> Option<&str> {
        self.message_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the message of a tag
    ///
    /// Returns None if there is no message
    pub fn message_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, raw::git_tag_message(&*self.raw)) }
    }

    /// Get the name of a tag
    ///
    /// Returns None if it is not valid utf8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes()).ok()
    }

    /// Get the name of a tag
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_tag_name(&*self.raw)).unwrap() }
    }

    /// Recursively peel a tag until a non tag git_object is found
    pub fn peel(&self) -> Result<Object<'repo>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tag_peel(&mut ret, &*self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get the tagger (author) of a tag
    ///
    /// If the author is unspecified, then `None` is returned.
    pub fn tagger(&self) -> Option<Signature<'_>> {
        unsafe {
            let ptr = raw::git_tag_tagger(&*self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(signature::from_raw_const(self, ptr))
            }
        }
    }

    /// Get the tagged object of a tag
    ///
    /// This method performs a repository lookup for the given object and
    /// returns it
    pub fn target(&self) -> Result<Object<'repo>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tag_target(&mut ret, &*self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get the OID of the tagged object of a tag
    pub fn target_id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_tag_target_id(&*self.raw)) }
    }

    /// Get the ObjectType of the tagged object of a tag
    pub fn target_type(&self) -> Option<ObjectType> {
        unsafe { ObjectType::from_raw(raw::git_tag_target_type(&*self.raw)) }
    }

    /// Casts this Tag to be usable as an `Object`
    pub fn as_object(&self) -> &Object<'repo> {
        unsafe { &*(self as *const _ as *const Object<'repo>) }
    }

    /// Consumes Tag to be returned as an `Object`
    pub fn into_object(self) -> Object<'repo> {
        assert_eq!(mem::size_of_val(&self), mem::size_of::<Object<'_>>());
        unsafe { mem::transmute(self) }
    }
}

impl<'repo> std::fmt::Debug for Tag<'repo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut ds = f.debug_struct("Tag");
        if let Some(name) = self.name() {
            ds.field("name", &name);
        }
        ds.field("id", &self.id());
        ds.finish()
    }
}

impl<'repo> Binding for Tag<'repo> {
    type Raw = *mut raw::git_tag;
    unsafe fn from_raw(raw: *mut raw::git_tag) -> Tag<'repo> {
        Tag {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_tag {
        self.raw
    }
}

impl<'repo> Clone for Tag<'repo> {
    fn clone(&self) -> Self {
        self.as_object().clone().into_tag().ok().unwrap()
    }
}

impl<'repo> Drop for Tag<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_tag_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use crate::Tag;

    // Reference -- https://git-scm.com/docs/git-check-ref-format
    #[test]
    fn name_is_valid() {
        assert_eq!(Tag::is_valid_name("blah_blah"), true);
        assert_eq!(Tag::is_valid_name("v1.2.3"), true);
        assert_eq!(Tag::is_valid_name("my/tag"), true);
        assert_eq!(Tag::is_valid_name("@"), true);

        assert_eq!(Tag::is_valid_name("-foo"), false);
        assert_eq!(Tag::is_valid_name("foo:bar"), false);
        assert_eq!(Tag::is_valid_name("foo^bar"), false);
        assert_eq!(Tag::is_valid_name("foo."), false);
        assert_eq!(Tag::is_valid_name("@{"), false);
        assert_eq!(Tag::is_valid_name("as\\cd"), false);
    }

    #[test]
    #[should_panic]
    fn is_valid_name_for_invalid_tag() {
        Tag::is_valid_name("ab\012");
    }

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let id = head.target().unwrap();
        assert!(repo.find_tag(id).is_err());

        let obj = repo.find_object(id, None).unwrap();
        let sig = repo.signature().unwrap();
        let tag_id = repo.tag("foo", &obj, &sig, "msg", false).unwrap();
        let tag = repo.find_tag(tag_id).unwrap();
        assert_eq!(tag.id(), tag_id);

        let tags = repo.tag_names(None).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags.get(0), Some("foo"));

        assert_eq!(tag.name(), Some("foo"));
        assert_eq!(tag.message(), Some("msg"));
        assert_eq!(tag.peel().unwrap().id(), obj.id());
        assert_eq!(tag.target_id(), obj.id());
        assert_eq!(tag.target_type(), Some(crate::ObjectType::Commit));

        assert_eq!(tag.tagger().unwrap().name(), sig.name());
        tag.target().unwrap();
        tag.into_object();

        repo.find_object(tag_id, None).unwrap().as_tag().unwrap();
        repo.find_object(tag_id, None)
            .unwrap()
            .into_tag()
            .ok()
            .unwrap();

        repo.tag_delete("foo").unwrap();
    }

    #[test]
    fn lite() {
        let (_td, repo) = crate::test::repo_init();
        let head = t!(repo.head());
        let id = head.target().unwrap();
        let obj = t!(repo.find_object(id, None));
        let tag_id = t!(repo.tag_lightweight("foo", &obj, false));
        assert!(repo.find_tag(tag_id).is_err());
        assert_eq!(t!(repo.refname_to_id("refs/tags/foo")), id);

        let tags = t!(repo.tag_names(Some("f*")));
        assert_eq!(tags.len(), 1);
        let tags = t!(repo.tag_names(Some("b*")));
        assert_eq!(tags.len(), 0);
    }
}
