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

use std::string::ToString;
use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};

use icecap_core::prelude::*;
use icecap_core::config::RingBufferConfig;
use icecap_core::logger::{Logger, Level, DisplayMode};
use icecap_core::finite_set::Finite;
use icecap_core::rpc_sel4::RPCClient;
use icecap_core::config::RingBufferKicksConfig;
use icecap_start_generic::declare_generic_main;
use icecap_event_server_types::calls::Client as EventServerRequest;
use icecap_event_server_types::events;

use veracruz_utils::platform::icecap::message::{Request, Response, Error};

use crate::managers::session_manager;

declare_generic_main!(main);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    event: Notification,
    event_server_endpoint: Endpoint,
    event_server_bitfield: usize,
    channel: RingBufferConfig,
}

fn main(config: Config) -> Fallible<()> {
    debug_println!("runtime manager enter");
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

    RuntimeManager::new(channel, config.event, config.event_server_bitfield).run()
}

struct RuntimeManager {
    channel: PacketRingBuffer,
    event: Notification,
    event_server_bitfield: usize,
    active: bool,
}

impl RuntimeManager {

    fn new(channel: RingBuffer, event: Notification, event_server_bitfield: usize) -> Self {
        channel.enable_notify_read();
        channel.enable_notify_write();
        Self {
            channel: PacketRingBuffer::new(channel),
            event,
            event_server_bitfield,
            active: true,
        }
    }

    fn run(&mut self) -> Fallible<()> {
        loop {
            let req = self.recv()?;
            let resp = self.handle(&req)?;
            self.send(&resp)?;
            if !self.active {
                std::icecap_impl::external::runtime::exit();
            }
        }
    }

    fn handle(&mut self, req: &Request) -> Fallible<Response> {
        Ok(match req {
            Request::New { policy_json } => {
                session_manager::init_session_manager(&policy_json).unwrap();
                Response::New
            }
            Request::GetEnclaveCert => {
                match session_manager::get_enclave_cert_pem() {
                    Err(s) => {
                        log::debug!("{}", s);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(cert) => {
                        Response::GetEnclaveCert(cert)
                    }
                }
            }
            Request::GetEnclaveName => {
                match session_manager::get_enclave_name() {
                    Err(s) => {
                        log::debug!("{}", s);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(name) => {
                        Response::GetEnclaveName(name)
                    }
                }
            }
            Request::NewTlsSession => {
                match session_manager::new_session() {
                    Err(s) => {
                        log::debug!("{}", s);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(sess) => {
                        Response::NewTlsSession(sess)
                    }
                }
            }
            Request::CloseTlsSession(sess) => {
                match session_manager::close_session(*sess) {
                    Err(s) => {
                        log::debug!("{}", s);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(()) => {
                        Response::CloseTlsSession
                    }
                }
            }
            Request::SendTlsData(sess, data) => {
                match session_manager::send_data(*sess, data) {
                    Err(s) => {
                        log::debug!("{}", s);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(()) => {
                        Response::SendTlsData
                    }
                }
            }
            Request::GetTlsDataNeeded(sess) => {
                match session_manager::get_data_needed(*sess) {
                    Err(s) => {
                        log::debug!("{}", s);
                        Response::Error(Error::Unspecified)
                    }
                    Ok(needed) => {
                        Response::GetTlsDataNeeded(needed)
                    }
                }
            }
            Request::GetTlsData(sess) => {
                match session_manager::get_data(*sess) {
                    Err(s) => {
                        log::debug!("{}", s);
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

    fn wait(&self) -> Fallible<()> {
        return Ok(());
        log::trace!("blocking");
        let bit_lots = self.event.wait();
        log::trace!("unblocked");
        for bit_lot_index in biterate::biterate(bit_lots) {
            let bit_lot = unsafe {
                &*((self.event_server_bitfield + ((8 * bit_lot_index) as usize)) as *const core::sync::atomic::AtomicU64)
            };
            let bits = bit_lot.swap(0, core::sync::atomic::Ordering::SeqCst);
            for bit in biterate::biterate(bits) {
                let in_index = (bit_lot_index * 64 + bit) as usize;
                debug_println!("in_index = {}", in_index);
            }
        }
        Ok(())
    }

    fn send(&mut self, resp: &Response) -> Fallible<()> {
        log::trace!("write: {:x?}", resp);
        let mut block = false;
        let resp_bytes = serialize(resp).unwrap();
        while !self.channel.write(&resp_bytes) {
            log::debug!("host ring buffer full, waiting on notification");
            if block {
                self.wait()?;
            } else {
                block = true;
                self.channel.enable_notify_write();
            }
        }
        self.channel.notify_write();
        Ok(())
    }

    fn recv(&mut self) -> Fallible<Request> {
        log::trace!("read enter");
        let mut block = false;
        loop {
            if let Some(msg) = self.channel.read() {
                self.channel.notify_read();
                let req = deserialize(&msg).unwrap();
                log::trace!("read: {:x?}", req);
                return Ok(req);
            } else if block {
                self.wait()?;
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
    logger.level = Level::Trace;
    // logger.level = Level::Debug;
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
