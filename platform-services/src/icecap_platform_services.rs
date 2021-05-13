//! IceCap-specific platform services
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

use core::sync::atomic::{AtomicU8, Ordering};
use crate::Result;

// HACK

static RNG_STATE: AtomicU8 = AtomicU8::new(0);

pub fn platform_getrandom(buffer: &mut [u8]) -> Result {
    for b in buffer {
        *b = RNG_STATE.fetch_add(1, Ordering::SeqCst);
    }
    Result::Success
}
