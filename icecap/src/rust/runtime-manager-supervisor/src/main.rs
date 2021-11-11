//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

#![no_std]

use serde::{Deserialize, Serialize};

use icecap_std::prelude::*,
use icecap_std::logger::{DisplayMode, Level, Logger},
use icecap_std::rpc_sel4::RPCClient,
use icecap_std::runtime as icecap_runtime,
use icecap_start_generic::declare_generic_main;

declare_generic_main!(main);

const LOG_LEVEL: Level = Level::Trace;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
}

fn init_logging() {
    let mut logger = Logger::default();
    logger.level = LOG_LEVEL;
    logger.display_mode = DisplayMode::Line;
    logger.write = |s| debug_println!("{}", s);
    logger.init().unwrap();
}

fn main(config: Config) -> Fallible<()> {
    init_logging();
    Ok(())
}
