use std::collections;

use icecap_core::prelude::*;
use icecap_core::config::*;
use icecap_start_generic::declare_generic_main;

use veracruz_utils::platform::icecap::message::{Request, Response};
use crate::managers::session_manager as actions;
use bincode::{serialize, deserialize};

use serde::{Serialize, Deserialize};

extern crate alloc;

#[no_mangle]
extern "C" fn fmodf(x: f32, y: f32) -> f32 {
    libm::fmodf(x, y)
}

#[no_mangle]
extern "C" fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}

declare_generic_main!(main);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    rb: RingBufferConfig,
}

fn main(config: Config) -> Fallible<()> {
    icecap_std_external::set_panic();
    std::icecap_impl::set_now(std::time::Duration::from_secs(1590968361));
    let rb = RingBuffer::realize_resume(&config.rb);
    let wait = config.rb.wait;
    run(rb, wait)
}

fn run(rb: RingBuffer, wait: Notification) -> Fallible<()> {
    let mut server = Server::new(rb, wait);
    println!("running veracruz runtime manager");
    server.run()
}

struct Server {
    rb: PacketRingBuffer,
    wait: Notification,
    active: bool,
}

impl Server {

    fn new(rb: RingBuffer, wait: Notification) -> Self {
        rb.enable_notify_read();
        rb.enable_notify_write();
        Self {
            rb: PacketRingBuffer::new(rb),
            wait,
            active: true,
        }
    }

    fn run(&mut self) -> Fallible<()> {
        loop {
            let req = self.recv()?;
            let resp = self.handle(&req)?;
            self.send(&resp)?;
            if !self.active {
                panic!("")
                // println!("shutting down...");
                // self.ctrl.send(MessageInfo::empty())
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
                let cert = actions::get_enclave_cert_pem().map_err(|s| format_err!("{}", s))?;
                Response::GetEnclaveCert(cert)
            }
            Request::GetEnclaveName => {
                let name = actions::get_enclave_name().map_err(|s| format_err!("{}", s))?;
                Response::GetEnclaveName(name)
            }
            Request::NewTlsSession => {
                let sess = actions::new_session().map_err(|s| format_err!("{}", s))?;
                Response::NewTlsSession(sess)
            }
            Request::CloseTlsSession(sess) => {
                actions::close_session(*sess).map_err(|s| format_err!("{}", s))?;
                Response::CloseTlsSession
            }
            Request::SendTlsData(sess, data) => {
                actions::send_data(*sess, data).map_err(|s| format_err!("{}", s))?;
                Response::SendTlsData
            }
            Request::GetTlsDataNeeded(sess) => {
                let needed = actions::get_data_needed(*sess).map_err(|s| format_err!("{}", s))?;
                Response::GetTlsDataNeeded(needed)
            }
            Request::GetTlsData(sess) => {
                let (active, data) = actions::get_data(*sess).map_err(|s| format_err!("{}", s))?;
                self.active = active;
                Response::GetTlsData(active, data)
            }
        })
    }

    fn send(&mut self, resp: &Response) -> Fallible<()> {
        let resp_bytes = serialize(resp).unwrap();
        while !self.rb.write(&resp_bytes) {
            panic!();
            // self.wait.wait();
        }
        self.rb.notify_write();
        Ok(())
    }

    fn recv(&mut self) -> Fallible<Request> {
        loop {
            if let Some(msg) = self.rb.read() {
                self.rb.notify_read();
                let req = deserialize(&msg).unwrap();
                return Ok(req);
            }
            self.wait.wait();
        }
    }

}
