#![allow(bad_style, improper_ctypes)]

extern crate libgit2_sys;
extern crate libc;

use libc::*;
use libgit2_sys::*;

include!(concat!(env!("OUT_DIR"), "/all.rs"));
