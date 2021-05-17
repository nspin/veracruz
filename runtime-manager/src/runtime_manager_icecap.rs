use std::collections;

use icecap_core::prelude::*;
use icecap_core::config::*;
use icecap_start_generic::declare_generic_main;

use veracruz_utils::platform::icecap::message::{Request, Response, Error};
use crate::managers::session_manager as actions;
use bincode::{serialize, deserialize};
use alloc::boxed::Box;
use alloc::string::ToString;
use log::{Level, Log, Metadata, Record, SetLoggerError};
use serde::{Serialize, Deserialize};

extern crate alloc;

declare_generic_main!(main);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    host_ring_buffer: RingBufferConfig,
}

fn main(config: Config) -> Fallible<()> {
    icecap_std_external::set_panic();
    std::icecap_impl::set_now(std::time::Duration::from_secs(1621182569)); // HACK
    init_log().unwrap();
    let host_ring_buffer = RingBuffer::realize_resume(&config.host_ring_buffer);
    let host_ring_buffer_notification = config.host_ring_buffer.wait;
    RuntimeManager::new(host_ring_buffer, host_ring_buffer_notification).run()
}

struct RuntimeManager {
    host_ring_buffer: PacketRingBuffer,
    host_ring_buffer_notification: Notification,
    active: bool,
}

impl RuntimeManager {

    fn new(host_ring_buffer: RingBuffer, host_ring_buffer_notification: Notification) -> Self {
        host_ring_buffer.enable_notify_read();
        host_ring_buffer.enable_notify_write();
        Self {
            host_ring_buffer: PacketRingBuffer::new(host_ring_buffer),
            host_ring_buffer_notification,
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
                unreachable!();
            }
        }
    }

    fn handle(&mut self, req: &Request) -> Fallible<Response> {
        Ok(match req {
            Request::New { policy_json } => {
                actions::init_session_manager(&policy_json).unwrap();
                Response::New
            }
            Request::GetEnclaveCert => {
                match actions::get_enclave_cert_pem() {
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
                match actions::get_enclave_name() {
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
                match actions::new_session() {
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
                match actions::close_session(*sess) {
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
                match actions::send_data(*sess, data) {
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
                match actions::get_data_needed(*sess) {
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
                match actions::get_data(*sess) {
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

    fn send(&mut self, resp: &Response) -> Fallible<()> {
        let resp_bytes = serialize(resp).unwrap();
        while !self.host_ring_buffer.write(&resp_bytes) {
            log::debug!("host ring buffer full, waiting on notification");
            self.host_ring_buffer_notification.wait();
        }
        self.host_ring_buffer.notify_write();
        Ok(())
    }

    fn recv(&mut self) -> Fallible<Request> {
        loop {
            if let Some(msg) = self.host_ring_buffer.read() {
                self.host_ring_buffer.notify_read();
                let req = deserialize(&msg).unwrap();
                return Ok(req);
            }
            self.host_ring_buffer_notification.wait();
        }
    }

}



struct SimpleLogger {
    level: Level,
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_string = {
                {
                    record.level().to_string()
                }
            };
            let target = if record.target().len() > 0 {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };
            {
                // println!("{:<5} [{}] {}", level_string, target, record.args());
                println!("{:<5} [{}:{}] {}",
                    level_string,
                    record.file().map(|x| x.to_string()).or(record.file_static().map(|x| x.to_string())).unwrap_or("?".to_string()),
                    record.line().map(|x| format!("{}", x)).unwrap_or("?".to_string()),
                    record.args(),
                );
            }
        }
    }

    fn flush(&self) {}
}

pub fn init_with_level(level: Level) -> Result<(), SetLoggerError> {
    let logger = SimpleLogger {
        level,
    };
    log::set_logger(Box::leak(Box::new(logger)))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

pub fn init_log() -> Result<(), SetLoggerError> {
    init_with_level(Level::Trace)
}

// HACK
#[no_mangle]
extern "C" fn fmodf(x: f32, y: f32) -> f32 {
    libm::fmodf(x, y)
}

// HACK
#[no_mangle]
extern "C" fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}
