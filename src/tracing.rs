use std::sync::atomic::{AtomicUsize, Ordering};

use libc::c_char;
use log::RecordBuilder;

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
            _ => panic!("Unknown git trace level"),
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

impl TraceLevel {
    /// Attempt to convert this [TraceLevel] to a [log::LevelFilter].
    ///
    /// This is done trivially with two exceptions:
    /// - [TraceLevel::None] goes to [None]
    /// - [TraceLevel::Fatal] goes to [log::Level::Error].
    pub const fn as_log_level(self) -> Option<log::Level> {
        use log::Level;

        match self {
            Self::None => None,
            Self::Fatal | Self::Error => Some(Level::Error),
            Self::Warn => Some(Level::Warn),
            Self::Info => Some(Level::Info),
            Self::Debug => Some(Level::Debug),
            Self::Trace => Some(Level::Trace),
        }
    }
}

//TODO: pass raw &[u8] and leave conversion to consumer (breaking API)
/// Callback type used to pass tracing events to the subscriber.
/// see `trace_set` to register a subscriber.
pub type TracingCb = fn(TraceLevel, &str);

static CALLBACK: AtomicUsize = AtomicUsize::new(0);

/// Set the tracing callback.
pub fn trace_set(level: TraceLevel, cb: TracingCb) -> bool {
    CALLBACK.store(cb as usize, Ordering::SeqCst);

    unsafe {
        raw::git_trace_set(level.raw(), Some(tracing_cb_c));
    }

    return true;
}

/// Passes [trace_set] a shim function to pass tracing info to the [log] crate.
pub fn trace_shim_log_crate() {
    // Use `trace` to get all tracing events -- let the user configure filtering
    // through the `log` crate.
    trace_set(TraceLevel::Trace, |level, msg| {
        // Convert the trace level to a log level.
        let log_level = level
            .as_log_level()
            .expect("libgit2 should not produce tracing events with level=None");

        // Build a record to pass to the logger.
        let mut record_builder = RecordBuilder::new();

        // Set the target and level.
        record_builder.target("libgit2").level(log_level);

        // Log the trace event to the global logger.
        log::logger().log(&record_builder.args(format_args!("{}", msg)).build());
    });
}

extern "C" fn tracing_cb_c(level: raw::git_trace_level_t, msg: *const c_char) {
    let cb = CALLBACK.load(Ordering::SeqCst);
    panic::wrap(|| unsafe {
        let cb: TracingCb = std::mem::transmute(cb);
        let msg = std::ffi::CStr::from_ptr(msg).to_string_lossy();
        cb(Binding::from_raw(level), msg.as_ref());
    });
}
