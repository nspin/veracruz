//! Veracruz test build script
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE_MIT.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

use std::process::Command;

fn main() {
    #[cfg(not(feature = "icecap"))]
    {
        Command::new("bash")
            .args(&["./populate-test-database.sh"])
            .output()
            .unwrap();
    }
}
