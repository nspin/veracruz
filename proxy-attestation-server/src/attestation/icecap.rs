use crate::error::*;

pub fn start(firmware_version: &str, device_id: i32) -> ProxyAttestationServerResponder {
    Ok(base64::encode(&[]))
}
