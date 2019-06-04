#![allow(bad_style, improper_ctypes)]

extern crate libc;
extern crate libgit2_sys;

use libc::*;
use libgit2_sys::*;

include!(concat!(env!("OUT_DIR"), "/all.rs"));
