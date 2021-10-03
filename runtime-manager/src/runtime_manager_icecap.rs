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
        let token = self.native_attestation(&challenge, &csr)?;
        Ok((token, csr))
    }

    fn native_attestation(&self, challenge: &[u8], csr: &[u8]) -> Result<Vec<u8>, RuntimeManagerError> {
        todo!()
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
    // logger.level = Level::Trace;
    logger.level = Level::Debug;
    // logger.level = Level::Info;
    logger.display_mode = DisplayMode::Line;
    logger.write = |s| debug_println!("{}", s);
    logger.init().unwrap();
}

// HACK
mod hack {

    #[no_mangle]
    extern "C" fn fmod(x: f64, y: f64) -> f64 {
        libm::fmod(x, y)
    }

    #[no_mangle]
    extern "C" fn fmodf(x: f32, y: f32) -> f32 {
        libm::fmodf(x, y)
    }
}
