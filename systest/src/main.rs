#![allow(bad_style, deprecated, improper_ctypes)]

use libc::*;
use libgit2_sys::*;

include!(concat!(env!("OUT_DIR"), "/all.rs"));
