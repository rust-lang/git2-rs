use std::any::Any;
use std::cell::RefCell;

// This is technically super unsafe, allowing capturing an arbitrary environment
// and then declaring it Send to cross the boundary into a safe call to `wrap`.
//
// One of the main reasons for the `Send` bound, however, is for exception
// safety mitigation. We do not want to impose exception safety concerns on
// consumers, so at the end of all calls to libgit2 we check if there was an
// error while calling a callback and then re-raise as a panic if necessary.
// Along these lines we simply catch an exception to be re-raised on the Rust
// side after passing back through C.
//
// As a result we're generally keeping the equivalent semantics for Rust, so
// this `unsafe impl Send` should be ok.
macro_rules! wrap_env {
    (fn $fn_name:ident($($arg:ident: $arg_t:ty),*) -> $ret:ty { $body:expr }
     returning $name:ident as $ret_expr:expr ) => {
        extern fn $fn_name($($arg: $arg_t),*) -> $ret {
            struct Env { $($arg: $arg_t),* }
            unsafe impl Send for Env {}
            let env = Env { $($arg: $arg),* };
            let $name = ::panic::wrap(move || {
                $(let $arg = env.$arg;)*
                $body
            });
            $ret_expr
        }
    }
}

thread_local!(static LAST_ERROR: RefCell<Option<Box<Any + Send>>> = {
    RefCell::new(None)
});

#[cfg(feature = "unstable")]
pub fn wrap<T, F: FnOnce() -> T + Send + 'static>(f: F) -> Option<T> {
    use std::thread;
    if LAST_ERROR.with(|slot| slot.borrow().is_some()) {
        return None
    }
    match thread::catch_panic(f) {
        Ok(ret) => Some(ret),
        Err(e) => {
            LAST_ERROR.with(move |slot| {
                *slot.borrow_mut() = Some(e);
            });
            None
        }
    }
}

#[cfg(not(feature = "unstable"))]
pub fn wrap<T, F: FnOnce() -> T + Send + 'static>(f: F) -> Option<T> {
    struct Bomb { enabled: bool }
    impl Drop for Bomb {
        fn drop(&mut self) {
            if !self.enabled { return }
            panic!("callback has panicked, and continuing to unwind into C \
                    is not safe, so aborting the process");

        }
    }
    let mut bomb = Bomb { enabled: true };
    let ret = Some(f());
    bomb.enabled = false;
    return ret;
}

pub fn check() {
    let err = LAST_ERROR.with(|slot| slot.borrow_mut().take());
    match err {
        Some(err) => panic!(err),
        None => {}
    }
}

pub fn panicked() -> bool {
    LAST_ERROR.with(|slot| slot.borrow().is_some())
}
