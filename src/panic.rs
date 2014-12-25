use std::any::Any;
use std::cell::RefCell;
use std::rt::unwind;

thread_local!(static LAST_ERROR: RefCell<Option<Box<Any + Send>>> = {
    RefCell::new(None)
});

pub fn wrap<T, F: FnOnce() -> T>(f: F) -> Option<T> {
    if LAST_ERROR.with(|slot| slot.borrow().is_some()) {
        return None
    }
    let mut ret = None;
    let err = {
        let ret = &mut ret;
        unsafe { unwind::try(move || { *ret = Some(f()); }) }
    };
    match err {
        Ok(()) => ret,
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
    match err {
        Some(err) => panic!(err),
        None => {}
    }
}
