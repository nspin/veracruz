use crate::error::*;
use rand::Rng;

pub fn start(firmware_version: &str, device_id: i32) -> ProxyAttestationServerResponder {
    let mut challenge: [u8; 32] = [0; 32];
    let mut rng = rand::thread_rng();

    rng.fill(&mut challenge);

    let serialized_attestation_init =
        transport_protocol::serialize_psa_attestation_init(&challenge, device_id)?;
    Ok(base64::encode(&serialized_attestation_init))
}
