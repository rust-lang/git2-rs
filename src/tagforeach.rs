//! git_tag_foreach support
//! see original: <https://libgit2.org/libgit2/#HEAD/group/tag/git_tag_foreach>

use crate::{panic, raw, util::Binding, Oid};
use libc::{c_char, c_int};
use raw::git_oid;
use std::ffi::{c_void, CStr};

/// boxed callback type
pub(crate) type TagForeachCB<'a> = Box<dyn FnMut(Oid, &[u8]) -> bool + 'a>;

/// helper type to be able to pass callback to payload
pub(crate) struct TagForeachData<'a> {
    /// callback
    pub(crate) cb: TagForeachCB<'a>,
}

/// c callback forwarding to rust callback inside `TagForeachData`
/// see original: <https://libgit2.org/libgit2/#HEAD/group/callback/git_tag_foreach_cb>
pub(crate) extern "C" fn tag_foreach_cb(
    name: *const c_char,
    oid: *mut git_oid,
    payload: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let id: Oid = Binding::from_raw(oid as *const _);

        let name = CStr::from_ptr(name);
        let name = name.to_bytes();

        let payload = &mut *(payload as *mut TagForeachData<'_>);
        let cb = &mut payload.cb;

        let res = cb(id, name);

        if res {
            0
        } else {
            -1
        }
    })
    .unwrap_or(-1)
}

#[cfg(test)]
mod tests {

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let id = head.target().unwrap();
        assert!(repo.find_tag(id).is_err());

        let obj = repo.find_object(id, None).unwrap();
        let sig = repo.signature().unwrap();
        let tag_id = repo.tag("foo", &obj, &sig, "msg", false).unwrap();

        let mut tags = Vec::new();
        repo.tag_foreach(|id, name| {
            tags.push((id, String::from_utf8(name.into()).unwrap()));
            true
        })
        .unwrap();

        assert_eq!(tags[0].0, tag_id);
        assert_eq!(tags[0].1, "refs/tags/foo");
    }
}
