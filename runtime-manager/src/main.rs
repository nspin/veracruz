//! The Runtime Manager enclave
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

#![cfg_attr(any(feature = "tz", feature = "icecap"), no_main)]
#![crate_name = "runtime_manager_enclave"]
#![feature(rustc_private)]

// TODO(nspin)
#![feature(format_args_nl)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_variables)]

#[cfg(feature = "tz")]
pub mod runtime_manager_trustzone;
#[cfg(feature = "tz")]
pub use crate::runtime_manager_trustzone::*;

#[cfg(feature = "icecap")]
pub mod runtime_manager_icecap;
#[cfg(feature = "icecap")]
pub use crate::runtime_manager_icecap::*;

pub mod managers;

#[cfg(feature = "nitro")]
pub mod runtime_manager_nitro;

#[cfg(feature = "nitro")]
fn main() -> Result<(), String> {
    runtime_manager_nitro::nitro_main()
        .map_err(|err| format!("Runtime Manager::main nitro_main returned error:{:?}", err))
}

#[cfg(x)]
#[path = "fdjklsa.rs"]
mod x;
