use std::any::Any;
use std::cell::RefCell;

thread_local!(static LAST_ERROR: RefCell<Option<Box<dyn Any + Send>>> = {
    RefCell::new(None)
});

pub fn wrap<T, F: FnOnce() -> T + std::panic::UnwindSafe>(f: F) -> Option<T> {
    use std::panic;
    if LAST_ERROR.with(|slot| slot.borrow().is_some()) {
        return None;
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

pub fn check() {
    let err = LAST_ERROR.with(|slot| slot.borrow_mut().take());
    if let Some(err) = err {
        std::panic::resume_unwind(err);
    }
}

pub fn panicked() -> bool {
    LAST_ERROR.with(|slot| slot.borrow().is_some())
}
