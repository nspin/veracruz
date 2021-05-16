use super::result;

use core::sync::atomic::{AtomicU8, Ordering};

static STATE: AtomicU8 = AtomicU8::new(0);

pub fn platform_getrandom(buffer: &mut [u8]) -> result::Result {
    for b in buffer {
        *b = STATE.fetch_add(1, Ordering::SeqCst);
    }
    result::Result::Success
}
