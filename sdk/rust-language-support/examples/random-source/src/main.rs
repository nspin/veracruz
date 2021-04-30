//! Test of the trusted random source platform service.
//!
//! ## Context
//!
//! Test program for random number generation.
//!
//! Inputs:                  0.
//! Assumed form of inputs:  Not applicable.
//! Ensured form of outputs: A Pinecone-encoded vector of four `u8` values taken
//!                          from a random source provided by the underlying
//!                          platform.
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Copyright
//!
//! See the file `LICENSING.markdown` in the Veracruz root directory for licensing
//! and copyright information.

use std::fs;
use rand::Rng;
///// Generates a four-element long random vector of `u8` values.  Fails if the
///// random source is unavailable or experiences an error.
//fn generate_random_vector() -> Result<Vec<u8>, i32> {
    //let mut buffer: Vec<u8> = vec![0; 4];
    //match host::getrandom(&mut buffer) {
        //host::HCallReturnCode::ErrorServiceUnavailable => return_code::fail_service_unavailable(),
        //host::HCallReturnCode::Success(_) => Ok(buffer),
        //_otherwise => return_code::fail_generic(),
    //}
//}

/// Entry point: generates a four-element long random vector of `u8` values and
/// writes this back as the result.
fn main() {
    let output = "/output";
    let bytes = rand::thread_rng().gen::<[u8; 32]>();
    let rst = match pinecone::to_vec(&bytes) {
        Err(_err) => panic!(),
        Ok(s) => s,
    };
    fs::write(output, rst).unwrap();
}
