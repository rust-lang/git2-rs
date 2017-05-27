use std::any::Any;
use std::cell::RefCell;

thread_local!(static LAST_ERROR: RefCell<Option<Box<Any + Send>>> = {
    RefCell::new(None)
});

#[cfg(feature = "unstable")]
pub fn wrap<T, F: FnOnce() -> T + ::std::panic::UnwindSafe>(f: F) -> Option<T> {
    use std::panic;
    if LAST_ERROR.with(|slot| slot.borrow().is_some()) {
        return None
    }
    match panic::catch_unwind(f) {
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
pub fn wrap<T, F: FnOnce() -> T>(f: F) -> Option<T> {
    struct Bomb {
        enabled: bool,
    }
    impl Drop for Bomb {
        fn drop(&mut self) {
            if !self.enabled {
                return
            }
            panic!("callback has panicked, and continuing to unwind into C \
                    is not safe, so aborting the process");

        }
    }
    let mut bomb = Bomb { enabled: true };
    let ret = Some(f());
    bomb.enabled = false;
    ret
}

pub fn check() {
    let err = LAST_ERROR.with(|slot| slot.borrow_mut().take());
    if let Some(err) = err {
        panic!(err)
    }
}

pub fn panicked() -> bool {
    LAST_ERROR.with(|slot| slot.borrow().is_some())
}
