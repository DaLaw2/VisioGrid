use std::sync::atomic::{AtomicBool, Ordering};

pub struct PreventReenter {
    flag: &'static AtomicBool,
}

impl PreventReenter {
    pub fn new(flag: &'static AtomicBool) -> Option<Self> {
        if flag.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            Some(PreventReenter {
                flag
            })
        } else {
            None
        }
    }
}

impl Drop for PreventReenter {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}
