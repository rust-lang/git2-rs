use std::fmt;
use std::hash::{sip, Hash};
use libc;

use raw;

pub struct Oid {
    raw: raw::git_oid,
}

impl Oid {
    pub unsafe fn from_raw(oid: *const raw::git_oid) -> Oid {
        Oid { raw: *oid }
    }
}

impl fmt::Show for Oid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dst = [0u8, ..raw::GIT_OID_HEXSZ + 1];
        unsafe {
            raw::git_oid_tostr(dst.as_mut_ptr() as *mut libc::c_char,
                               dst.len() as libc::size_t, &self.raw);
        }
        f.write(dst.slice_to(dst.iter().position(|&a| a == 0).unwrap()))
    }
}

impl PartialEq for Oid {
    fn eq(&self, other: &Oid) -> bool {
        unsafe { raw::git_oid_equal(&self.raw, &other.raw) != 0 }
    }
}
impl Eq for Oid {}

impl PartialOrd for Oid {
    fn partial_cmp(&self, other: &Oid) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Oid {
    fn cmp(&self, other: &Oid) -> Ordering {
        match unsafe { raw::git_oid_cmp(&self.raw, &other.raw) } {
            0 => Equal,
            n if n < 0 => Less,
            _ => Greater,
        }
    }
}

impl Clone for Oid {
    fn clone(&self) -> Oid { *self }
}

impl Hash for Oid {
    fn hash(&self, into: &mut sip::SipState) {
        self.raw.id.as_slice().hash(into)
    }
}
