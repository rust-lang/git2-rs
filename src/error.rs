use std::fmt;
use std::c_str::CString;

use raw;
use libc;

pub struct Error {
    raw: raw::git_error,
}

impl Error {
    pub fn last_error() -> Option<Error >{
        let mut ret = Error {
            raw: raw::git_error {
                message: 0 as *mut libc::c_char,
                klass: 0,
            }
        };
        if unsafe { raw::giterr_detach(&mut ret.raw) } == 0 {
            Some(ret)
        } else {
            None
        }
    }
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "[{}] ", self.raw.klass));
        let cstr = unsafe { CString::new(self.raw.message as *const _, false) };
        f.write(cstr.as_bytes_no_nul())
    }
}
