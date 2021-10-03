//! IceCap-specific material for the Runtime Manager enclave
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

extern crate alloc;

use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};

use icecap_core::{
    prelude::*,
    config::RingBufferConfig,
    logger::{Logger, Level, DisplayMode},
    finite_set::Finite,
    rpc_sel4::RPCClient,
    config::RingBufferKicksConfig,
};
use icecap_start_generic::declare_generic_main;
use icecap_event_server_types::{
    events,
    calls::Client as EventServerRequest,
    Bitfield as EventServerBitfield,
};

use veracruz_utils::platform::icecap::message::{Request, Response, Error};

use crate::managers::{session_manager, RuntimeManagerError};

declare_generic_main!(main);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    event: Notification,
    event_server_endpoint: Endpoint,
    event_server_bitfield: usize,
    channel: RingBufferConfig,
}

fn main(config: Config) -> Fallible<()> {
    icecap_runtime_init();
    log::debug!("runtime manager enter");

    let channel = {
        let event_server = RPCClient::<EventServerRequest>::new(config.event_server_endpoint);
        let index = {
            use events::*;
            RealmOut::RingBuffer(RealmRingBufferOut::Host(RealmRingBufferId::Channel))
        };
        let kick = Box::new(move || event_server.call::<()>(&EventServerRequest::Signal {
            index: index.to_nat(),
        }));
        RingBuffer::realize_resume(
            &config.channel,
            RingBufferKicksConfig {
                read: kick.clone(),
                write: kick,
            },
        )
    };

    let event_server_bitfield = unsafe {
        EventServerBitfield::new(config.event_server_bitfield)
    };

    event_server_bitfield.clear_ignore_all();

    RuntimeManager::new(channel, config.event, event_server_bitfield).run()
}

struct RuntimeManager {
    channel: PacketRingBuffer,
    event: Notification,
    event_server_bitfield: EventServerBitfield,
    active: bool,
}

impl RuntimeManager {

    fn new(channel: RingBuffer, event: Notification, event_server_bitfield: EventServerBitfield) -> Self {
        Self {
            channel: PacketRingBuffer::new(channel),
            event,
            event_server_bitfield,
            active: true,
        }
    }

    fn run(&mut self) -> Fallible<()> {
        loop {
            let req = self.recv().map_err(|e| format_err!("RuntimeManagerErro: {:?}", e))?;
            let resp = self.handle(req)?;
            self.send(&resp).map_err(|e| format_err!("RuntimeManagerErro: {:?}", e))?;
            if !self.active {
                std::icecap_impl::external::runtime::exit(); // HACK
            }
        }
    }

    fn handle(&mut self, req: Request) -> Fallible<Response> {
        Ok(match req {
            Request::Initialize { policy_json } => {
                match session_manager::init_session_manager().and(session_manager::load_policy(&policy_json)) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(()) => {
                        Response::Initialize
                    }
                }
            }
            Request::Attestation { device_id, challenge } => {
                match self.handle_attestation(device_id, &challenge) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok((token, csr)) => {
                        Response::Attestation { token, csr }
                    }
                }
            }
            Request::CertificateChain { root_cert, compute_cert } => {
                match session_manager::load_cert_chain(&vec![compute_cert, root_cert]) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(()) => {
                        Response::CertificateChain
                    }
                }
            }
            Request::NewTlsSession => {
                match session_manager::new_session() {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(sess) => {
                        Response::NewTlsSession(sess)
                    }
                }
            }
            Request::CloseTlsSession(sess) => {
                match session_manager::close_session(sess) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(()) => {
                        Response::CloseTlsSession
                    }
                }
            }
            Request::SendTlsData(sess, data) => {
                match session_manager::send_data(sess, &data) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(()) => {
                        Response::SendTlsData
                    }
                }
            }
            Request::GetTlsDataNeeded(sess) => {
                match session_manager::get_data_needed(sess) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(needed) => {
                        Response::GetTlsDataNeeded(needed)
                    }
                }
            }
            Request::GetTlsData(sess) => {
                match session_manager::get_data(sess) {
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Response::Error(Error::Unspecified)
                    }
                    Ok((active, data)) => {
                        self.active = active;
                        Response::GetTlsData(active, data)
                    }
                }
            }
        })
    }

    fn handle_attestation(&self, device_id: i32, challenge: &[u8]) -> Result<(Vec<u8>, Vec<u8>), RuntimeManagerError> {
        let csr = session_manager::generate_csr()?;
        let token = attestation_hack::native_attestation(&challenge, &csr)?;
        Ok((token, csr))
    }

    fn wait(&self) {
        log::trace!("waiting");
        let badge = self.event.wait();
        log::trace!("done waiting");
        self.event_server_bitfield.clear_ignore(badge);
    }

    fn send(&mut self, resp: &Response) -> Result<(), RuntimeManagerError> {
        log::trace!("write: {:x?}", resp);
        let mut block = false;
        let resp_bytes = serialize(resp).map_err(RuntimeManagerError::SerializationError)?;
        while !self.channel.write(&resp_bytes) {
            log::warn!("host ring buffer full, waiting on notification");
            if block {
                self.wait();
                block = false;
            } else {
                block = true;
                self.channel.enable_notify_write();
            }
        }
        self.channel.notify_write();
        Ok(())
    }

    fn recv(&mut self) -> Result<Request, RuntimeManagerError> {
        log::trace!("read enter");
        let mut block = false;
        loop {
            if let Some(msg) = self.channel.read() {
                self.channel.notify_read();
                let req = deserialize(&msg).map_err(RuntimeManagerError::SerializationError)?;
                log::trace!("read: {:x?}", req);
                return Ok(req);
            } else if block {
                self.wait();
                block = false;
            } else {
                block = true;
                self.channel.enable_notify_read();
            }
        }
    }

}

const NOW: u64 = include!("../../icecap/build/NOW");

fn icecap_runtime_init() {
    icecap_std_external::set_panic();
    std::icecap_impl::set_now(std::time::Duration::from_secs(NOW)); // HACK
    let mut logger = Logger::default();
    logger.level = Level::Debug;
    logger.display_mode = DisplayMode::Line;
    logger.write = |s| debug_println!("{}", s);
    logger.init().unwrap();
}

// HACK
mod c_hack {

    use core::mem::MaybeUninit;
    use super::alloc::boxed::Box;

    #[no_mangle]
    extern "C" fn fmod(x: f64, y: f64) -> f64 {
        libm::fmod(x, y)
    }

    #[no_mangle]
    extern "C" fn fmodf(x: f32, y: f32) -> f32 {
        libm::fmodf(x, y)
    }

    #[no_mangle]
    extern "C" fn calloc(nelem: usize, elsize: usize) -> *mut core::ffi::c_void {
        let bytes = nelem * elsize;
        let ret: *mut [MaybeUninit<u8>] = Box::into_raw(Box::<[u8]>::new_uninit_slice(bytes));
        ret as *mut core::ffi::c_void
    }

    #[no_mangle]
    extern "C" fn free(ptr: *mut core::ffi::c_void) {
        unsafe {
            // TODO is this sound?
            Box::<u8>::from_raw(ptr as *mut u8);
        }
    }
}

// HACK
mod attestation_hack {

    use super::RuntimeManagerError;
    use ring::digest;

    const EXAMPLE_PRIVATE_KEY: [u8; 32] = [
        0xe6, 0xbf, 0x1e, 0x3d, 0xb4, 0x45, 0x42, 0xbe,
        0xf5, 0x35, 0xe7, 0xac, 0xbc, 0x2d, 0x54, 0xd0,
        0xba, 0x94, 0xbf, 0xb5, 0x47, 0x67, 0x2c, 0x31,
        0xc1, 0xd4, 0xee, 0x1c, 0x05, 0x76, 0xa1, 0x44,
    ];

    const EXAMPLE_HASH: [u8; 32] = [
        0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef,
        0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef,
        0xf0, 0x0d, 0xca, 0xfe, 0xf0, 0x0d, 0xca, 0xfe,
        0xf0, 0x0d, 0xca, 0xfe, 0xf0, 0x0d, 0xca, 0xfe,
    ];

    const ROOT_PRIVATE_KEY: &[u8] = &EXAMPLE_PRIVATE_KEY;

    const RUNTIME_MANAGER_HASH: &[u8] = &EXAMPLE_HASH;


    pub(super) fn native_attestation(challenge: &[u8], csr: &[u8]) -> Result<Vec<u8>, RuntimeManagerError> {

        let root_private_key = &ROOT_PRIVATE_KEY;
        let enclave_hash = &RUNTIME_MANAGER_HASH;
        let csr_hash = digest::digest(&digest::SHA256, csr);

        let mut root_key_handle: u16 = 0;
        assert_eq!(0, unsafe {
            psa_attestation::psa_initial_attest_load_key(
                root_private_key.as_ptr(),
                root_private_key.len() as u64,
                &mut root_key_handle,
            )
        });

        let mut token: Vec<u8> = Vec::with_capacity(2048);
        let mut token_len: u64 = 0;
        assert_eq!(0, unsafe {
            psa_attestation::psa_initial_attest_get_token(
                enclave_hash.as_ptr() as *const u8,
                enclave_hash.len() as u64,
                csr_hash.as_ref().as_ptr() as *const u8,
                csr_hash.as_ref().len() as u64,
                std::ptr::null() as *const u8,
                0,
                challenge.as_ptr() as *const u8,
                challenge.len() as u64,
                token.as_mut_ptr() as *mut u8,
                token.capacity() as u64,
                &mut token_len as *mut u64,
            )
        });
        unsafe {
            token.set_len(token_len as usize)
        };

        assert_eq!(0, unsafe {
            psa_attestation::psa_initial_attest_remove_key(
                root_key_handle,
            )
        });

        Ok(token)
    }

}

#[cfg(x)]
mod attestation_hack_old {

    use once_cell::sync::OnceCell;
    use super::Result;

    const ROOT_HASH: &[u8] = &EXAMPLE_HASH;

    const DEVICE_PRIVATE_KEY: &[u8] = &EXAMPLE_PRIVATE_KEY;

    const FIRMWARE_VERSION: &str = "0.3.0";

    static DEVICE_ID: OnceCell<i32> = OnceCell::new();

    fn get_device_public_key() -> Vec<u8> {
        let device_private_key = &DEVICE_PRIVATE_KEY;
        let mut device_key_handle: u16 = 0;
        assert_eq!(0, unsafe {
            psa_attestation::psa_initial_attest_load_key(
                device_private_key.as_ptr(),
                device_private_key.len() as u64,
                &mut device_key_handle,
            )
        });
        let mut device_public_key = std::vec::Vec::with_capacity(128);
        let mut device_public_key_size: u64 = 0;
        assert_eq!(0, unsafe {
            psa_attestation::t_cose_sign1_get_verification_pubkey(
                device_key_handle,
                device_public_key.as_mut_ptr() as *mut u8,
                device_public_key.capacity() as u64,
                &mut device_public_key_size as *mut u64,
            )
        });
        unsafe {
            device_public_key.set_len(device_public_key_size as usize)
        };
        device_public_key
    }

    pub(super) fn native_attestation(proxy_attestation_server_url: &str) -> Result<i32> {
        Ok(*DEVICE_ID.get_or_init(|| {
            native_attestation_once(proxy_attestation_server_url).unwrap() // HACK
        }))
    }

    fn native_attestation_once(proxy_attestation_server_url: &str) -> Result<i32> {
        let proxy_attestation_server_response = crate::send_proxy_attestation_server_start(proxy_attestation_server_url, "psa", FIRMWARE_VERSION)?;
        assert!(proxy_attestation_server_response.has_psa_attestation_init());
        let (challenge, device_id) = transport_protocol::parse_psa_attestation_init(
            proxy_attestation_server_response.get_psa_attestation_init(),
        )?;

        let root_hash = ROOT_HASH;

        let token = {
            let mut token: Vec<u8> = Vec::with_capacity(2048);
            let mut token_len: u64 = 0;
            let device_public_key_hash = ring::digest::digest(&ring::digest::SHA256, &get_device_public_key());
            let enclave_name = "ac40a0c".as_bytes(); // HACK
            assert_eq!(0, unsafe {
                psa_attestation::psa_initial_attest_get_token(
                    root_hash.as_ptr() as *const u8,
                    root_hash.len() as u64,
                    device_public_key_hash.as_ref().as_ptr() as *const u8,
                    device_public_key_hash.as_ref().len() as u64,
                    enclave_name.as_ptr() as *const u8,
                    enclave_name.len() as u64,
                    challenge.as_ptr() as *const u8,
                    challenge.len() as u64,
                    token.as_mut_ptr() as *mut u8,
                    2048,
                    &mut token_len as *mut u64,
                )
            });
            unsafe {
                token.set_len(token_len as usize)
            };
            token
        };

        let proxy_attestation_server_request = transport_protocol::serialize_native_psa_attestation_token(&token, device_id)?;
        let encoded_str = base64::encode(&proxy_attestation_server_request);
        let url = format!("{:}/PSA/AttestationToken", proxy_attestation_server_url);
        crate::post_buffer(&url, &encoded_str)?;

        Ok(device_id)
    }

    pub(super) fn proxy_attesation(challenge: &[u8], enclave_cert: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let enclave_hash = &RUNTIME_MANAGER_HASH;

        let token = {
            let mut token: Vec<u8> = Vec::with_capacity(2048);
            let mut token_len: u64 = 0;
            let enclave_cert_hash = ring::digest::digest(&ring::digest::SHA256, &enclave_cert);
            let enclave_name = "ac40a0c".as_bytes();
            assert_eq!(0, unsafe {
                psa_attestation::psa_initial_attest_get_token(
                    enclave_hash.as_ptr() as *const u8,
                    enclave_hash.len() as u64,
                    enclave_cert_hash.as_ref().as_ptr() as *const u8,
                    enclave_cert_hash.as_ref().len() as u64,
                    enclave_name.as_ptr() as *const u8,
                    enclave_name.len() as u64,
                    challenge.as_ptr() as *const u8,
                    challenge.len() as u64,
                    token.as_mut_ptr() as *mut u8,
                    2048,
                    &mut token_len as *mut u64,
                )
            });
            unsafe {
                token.set_len(token_len as usize)
            };
            token
        };

        Ok((token, get_device_public_key()))
    }
}
