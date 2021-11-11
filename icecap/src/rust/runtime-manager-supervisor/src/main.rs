//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

#![no_std]
#![no_main]
#![feature(format_args_nl)]

use serde::{Deserialize, Serialize};

use icecap_std::prelude::*;
use icecap_std::logger::{DisplayMode, Level, Logger};
use icecap_std::rpc_sel4::RPCClient;
use icecap_std::runtime as icecap_runtime;
use icecap_start_generic::declare_generic_main;

declare_generic_main!(main);

const LOG_LEVEL: Level = Level::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    ep: Endpoint,
    request_badge: Badge,
    fault_badge: Badge,
    mmap_base: u64,
    pool: Pool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pool {
    large_pages: Vec<LargePage>,
    hack_large_pages: Vec<LargePage>,
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
    debug_println!("hello {:x?}", config);
    Ok(())
}
