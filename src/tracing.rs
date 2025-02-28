use std::{
    ffi::CStr,
    sync::atomic::{AtomicPtr, Ordering},
};

use libc::{c_char, c_int};

use crate::{raw, util::Binding, Error};

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

/// Callback type used to pass tracing events to the subscriber.
/// see `trace_set` to register a subscriber.
pub type TracingCb = fn(TraceLevel, &[u8]);

/// Use an atomic pointer to store the global tracing subscriber function.
static CALLBACK: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

/// Set the global subscriber called when libgit2 produces a tracing message.
pub fn trace_set(level: TraceLevel, cb: TracingCb) -> Result<(), Error> {
    // Store the callback in the global atomic.
    CALLBACK.store(cb as *mut (), Ordering::SeqCst);

    // git_trace_set returns 0 if there was no error.
    let return_code: c_int = unsafe { raw::git_trace_set(level.raw(), Some(tracing_cb_c)) };

    if return_code != 0 {
        Err(Error::last_error(return_code))
    } else {
        Ok(())
    }
}

/// The tracing callback we pass to libgit2 (C ABI compatible).
extern "C" fn tracing_cb_c(level: raw::git_trace_level_t, msg: *const c_char) {
    // Load the callback function pointer from the global atomic.
    let cb: *mut () = CALLBACK.load(Ordering::SeqCst);

    // Transmute the callback pointer into the function pointer we know it to be.
    //
    // SAFETY: We only ever set the callback pointer with something cast from a TracingCb
    // so transmuting back to a TracingCb is safe. This is notably not an integer-to-pointer
    // transmute as described in the mem::transmute documentation and is in-line with the
    // example in that documentation for casing between *const () to fn pointers.
    let cb: TracingCb = unsafe { std::mem::transmute(cb) };

    // If libgit2 passes us a message that is null, drop it and do not pass it to the callback.
    // This is to avoid ever exposing rust code to a null ref, which would be Undefined Behavior.
    if msg.is_null() {
        return;
    }

    // Convert the message from a *const c_char to a &[u8] and pass it to the callback.
    //
    // SAFETY: We've just checked that the pointer is not null. The other safety requirements are left to
    // libgit2 to enforce -- namely that it gives us a valid, nul-terminated, C string, that that string exists
    // entirely in one allocation, that the string will not be mutated once passed to us, and that the nul-terminator is
    // within isize::MAX bytes from the given pointers data address.
    let msg: &CStr = unsafe { CStr::from_ptr(msg) };

    // Convert from a CStr to &[u8] to pass to the rust code callback.
    let msg: &[u8] = CStr::to_bytes(msg);

    // Do not bother with wrapping any of the following calls in `panic::wrap`:
    //
    // The previous implementation used `panic::wrap` here but never called `panic::check` to determine if the
    // trace callback had panicked, much less what caused it.
    //
    // This had the potential to lead to lost errors/unwinds, confusing to debugging situations, and potential issues
    // catching panics in other parts of the `git2-rs` codebase.
    //
    // Instead, we simply call the next two lines, both of which may panic, directly. We can rely on the
    // `extern "C"` semantics to appropriately catch the panics generated here and abort the process:
    //
    // Per <https://doc.rust-lang.org/std/panic/fn.catch_unwind.html>:
    // > Rust functions that are expected to be called from foreign code that does not support
    // > unwinding (such as C compiled with -fno-exceptions) should be defined using extern "C", which ensures
    // > that if the Rust code panics, it is automatically caught and the process is aborted. If this is the desired
    // > behavior, it is not necessary to use catch_unwind explicitly. This function should instead be used when
    // > more graceful error-handling is needed.

    // Convert the raw trace level into a type we can pass to the rust callback fn.
    //
    // SAFETY: Currently the implementation of this function (above) may panic, but is only marked as unsafe to match
    // the trait definition, thus we can consider this call safe.
    let level: TraceLevel = unsafe { Binding::from_raw(level) };

    // Call the user-supplied callback (which may panic).
    (cb)(level, msg);
}

#[cfg(test)]
mod tests {
    use super::TraceLevel;

    // Test that using the above function to set a tracing callback doesn't panic.
    #[test]
    fn smoke() {
        super::trace_set(TraceLevel::Trace, |level, msg| {
            dbg!(level, msg);
        })
        .expect("libgit2 can set global trace callback");
    }
}
