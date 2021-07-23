use lazy_static::lazy_static;
use std::sync::Mutex;

use libc::c_char;

use crate::{panic, raw, util::Binding};

/// Available tracing levels.  When tracing is set to a particular level,
/// callers will be provided tracing at the given level and all lower levels.
#[derive(Copy, Clone, Debug)]
pub enum TraceLevel {
    /// No tracing will be performed.
    None,

    /// Severe errors that may impact the program's execution
    Fatal,

    /// Errors that do not impact the program's execution
    Error,

    /// Warnings that suggest abnormal data
    Warn,

    /// Informational messages about program execution
    Info,

    /// Detailed data that allows for debugging
    Debug,

    /// Exceptionally detailed debugging data
    Trace,
}

impl Binding for TraceLevel {
    type Raw = raw::git_trace_level_t;
    unsafe fn from_raw(raw: raw::git_trace_level_t) -> Self {
        match raw {
            raw::GIT_TRACE_NONE => Self::None,
            raw::GIT_TRACE_FATAL => Self::Fatal,
            raw::GIT_TRACE_ERROR => Self::Error,
            raw::GIT_TRACE_WARN => Self::Warn,
            raw::GIT_TRACE_INFO => Self::Info,
            raw::GIT_TRACE_DEBUG => Self::Debug,
            raw::GIT_TRACE_TRACE => Self::Trace,
            _ => panic!("Unknown git diff binary kind"),
        }
    }
    fn raw(&self) -> raw::git_trace_level_t {
        match *self {
            Self::None => raw::GIT_TRACE_NONE,
            Self::Fatal => raw::GIT_TRACE_FATAL,
            Self::Error => raw::GIT_TRACE_ERROR,
            Self::Warn => raw::GIT_TRACE_WARN,
            Self::Info => raw::GIT_TRACE_INFO,
            Self::Debug => raw::GIT_TRACE_DEBUG,
            Self::Trace => raw::GIT_TRACE_TRACE,
        }
    }
}

pub type TracingCb = Box<dyn FnMut(TraceLevel, &str) + Sync + Send>;

lazy_static! {
    static ref CALLBACK: Mutex<Option<TracingCb>> = Mutex::new(None);
}

///
pub fn trace_set<T>(level: TraceLevel, cb: T) -> bool
where
    T: FnMut(TraceLevel, &str) + Sync + Send + 'static,
{
    if let Ok(mut static_cb) = CALLBACK.lock() {
        *static_cb = Some(Box::new(cb));

        unsafe {
            raw::git_trace_set(level.raw(), Some(tracing_cb_c));
        }

        return true;
    }

    false
}

extern "C" fn tracing_cb_c(level: raw::git_trace_level_t, msg: *const c_char) {
    panic::wrap(|| unsafe {
        if let Ok(mut cb) = CALLBACK.lock() {
            if let Some(cb) = cb.as_mut() {
                let msg = std::ffi::CStr::from_ptr(msg).to_str().unwrap();
                (*cb)(Binding::from_raw(level), msg);
            }
        }
    });
}
