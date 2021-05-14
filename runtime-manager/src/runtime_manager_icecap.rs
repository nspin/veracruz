#![feature(format_args_nl)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections;

use icecap_core::prelude::*;
use icecap_core::config::*;
use icecap_start_generic::declare_generic_main;

use serde::{Serialize, Deserialize};

declare_generic_main!(main);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
}

fn main(config: Config) -> Fallible<()> {
    println!("hello world");
    Ok(())
}
