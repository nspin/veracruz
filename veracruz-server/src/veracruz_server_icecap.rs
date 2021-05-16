// use crate::ec2_instance::EC2Instance;
// use crate::veracruz_server::VeracruzServer;
// use crate::veracruz_server::VeracruzServerError;
// use lazy_static::lazy_static;
// use std::sync::Mutex;
// use veracruz_utils::{
//     platform::{Platform, nitro::{RuntimeManagerMessage, NitroEnclave, NitroError, NitroStatus},
//     policy::policy::Policy,
// };

use std::env;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::mem::size_of;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::result;
use std::sync::Mutex;
use std::string::ToString;
use std::process::Command;

use crate::veracruz_server::VeracruzServer;
use crate::veracruz_server::VeracruzServerError;
use veracruz_utils::{
    policy::policy::Policy,
    platform::{
        Platform,
        icecap::message::{Request, Response, Header},
    },
};
use bincode::{serialize, deserialize};

const ENDPOINT_ENV: &str = "VERACRUZ_SERVER_ENDPOINT";

const UNEXPECTED_RESPONSE: &str = "unexpected response";
const NOT_IMPLEMENTED: &str = "not implemented";

type Result<T> = result::Result<T, VeracruzServerError>;

fn mk_err<T>(msg: impl ToString) -> Result<T> {
    Err(VeracruzServerError::DirectStrError(msg.to_string()))
}

fn mk_err_raw(msg: impl ToString) -> VeracruzServerError {
    VeracruzServerError::DirectStrError(msg.to_string())
}

enum Endpoint {
    TCP(SocketAddr),
    File(PathBuf),
}

impl Endpoint {

    fn parse(s: &str) -> Option<Self> {
        let it: Vec<&str> = s.splitn(2, ":").collect();
        if it.len() != 2 {
            return None
        }
        Some(match it[0] {
            "tcp" => {
                Endpoint::TCP(it[1].parse().ok()?)
            }
            "file" => {
                Endpoint::File(PathBuf::from(it[1]))
            }
            _ => {
                return None
            }
        })
    }

    fn realize(&self) -> Result<Box<dyn ReadWrite + Send>> {
        Ok(match self {
            Endpoint::TCP(addr) => {
                Box::new(TcpStream::connect(addr).map_err(mk_err_raw)?)
            }
            Endpoint::File(path) => {
                Box::new(OpenOptions::new().read(true).write(true).open(path).map_err(mk_err_raw)?)
            }
        })
    }

}

// HACK
// trait object can only be constrained by a single non-auto trait, so we must
// intersect Read and Write in another trait

trait ReadWrite: Read + Write {}

impl<T> ReadWrite for T where T: Read + Write {}

fn get_endpoint() -> Result<Box<dyn ReadWrite + Send>> {
    let s = env::var(ENDPOINT_ENV).map_err(mk_err_raw)?;
    Endpoint::parse(&s)
        .ok_or(VeracruzServerError::DirectStrError(format!("invalid value for {}: {:?}", ENDPOINT_ENV, s)))?
        .realize()
}

pub struct VeracruzServerIceCap {
    stream: Mutex<Box<dyn ReadWrite + Send>>,
}

impl VeracruzServer for VeracruzServerIceCap {

    fn new(policy_json: &str) -> Result<Self> {
        let status = Command::new("icecap-host")
            .arg("create")
            .arg("0")
            .arg("/spec.bin")
            .arg("file:/dev/rb_resource_server")
            .status().unwrap();
        assert!(status.success());
        let status = Command::new("icecap-host")
            .arg("hack-run")
            .arg("0")
            .status().unwrap();
        assert!(status.success());

        let handle = Self {
            stream: Mutex::new(get_endpoint()?),
        };
        match handle.send(&Request::New { policy_json: policy_json.to_string() })? {
            Response::New => (),
            _ => return mk_err(UNEXPECTED_RESPONSE),
        }
        Ok(handle)
    }

    fn proxy_psa_attestation_get_token(
        &mut self,
        challenge: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<u8>, i32)> {
        let (token, public_key, device_id) = (vec![], vec![], 0);
        return Ok((token, public_key, device_id));
    }

    fn plaintext_data(&mut self, data: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let parsed = transport_protocol::parse_runtime_manager_request(&data)?;

        if parsed.has_request_proxy_psa_attestation_token() {
            let rpat = parsed.get_request_proxy_psa_attestation_token();
            let challenge = transport_protocol::parse_request_proxy_psa_attestation_token(rpat);
            let (psa_attestation_token, pubkey, device_id) =
                self.proxy_psa_attestation_get_token(challenge)?;
            let serialized_pat = transport_protocol::serialize_proxy_psa_attestation_token(
                &psa_attestation_token,
                &pubkey,
                device_id,
            )?;
            Ok(Some(serialized_pat))
        } else {
            Err(VeracruzServerError::MissingFieldError(
                "plaintext_data proxy_psa_attestation_token",
            ))
        }
    }

    fn get_enclave_cert(&mut self) -> Result<Vec<u8>> {
       match self.send(&Request::GetEnclaveCert)? {
           Response::GetEnclaveCert(cert) => Ok(cert),
           _ => mk_err(UNEXPECTED_RESPONSE),
       }
    }

    fn get_enclave_name(&mut self) -> Result<String> {
       match self.send(&Request::GetEnclaveName)? {
           Response::GetEnclaveName(name) => Ok(name),
           _ => mk_err(UNEXPECTED_RESPONSE),
       }
    }

    fn new_tls_session(&mut self) -> Result<u32> {
       match self.send(&Request::NewTlsSession)? {
           Response::NewTlsSession(session_id) => Ok(session_id),
           _ => mk_err(UNEXPECTED_RESPONSE),
       }
    }

    fn close_tls_session(&mut self, session_id: u32) -> Result<()> {
       match self.send(&Request::CloseTlsSession(session_id))? {
           Response::CloseTlsSession => Ok(()),
           _ => mk_err(UNEXPECTED_RESPONSE),
       }
    }

    fn tls_data(&mut self, session_id: u32, input: Vec<u8>) -> Result<(bool, Option<Vec<Vec<u8>>>)> {
        match self.send(&Request::SendTlsData(session_id, input))? {
            Response::SendTlsData => (),
            _ => return mk_err(UNEXPECTED_RESPONSE),
        }

        let mut acc = Vec::new();
        let active = loop {
            if !self.tls_data_needed(session_id)? {
                break true;
            }
            match self.send(&Request::GetTlsData(session_id))? {
                Response::GetTlsData(active, data) => {
                    acc.push(data);
                    if !active {
                        break false;
                    }
                }
                _ => return mk_err(UNEXPECTED_RESPONSE),
            };
        };

        Ok((active, match acc.len() {
            0 => None,
            _ => Some(acc),
        }))
    }

    fn close(&mut self) -> Result<bool> {
        let status = Command::new("icecap-host")
            .arg("destroy")
            .arg("0")
            .status().unwrap();
        assert!(status.success());
        Ok(true)
    }
}

impl Drop for VeracruzServerIceCap {
    fn drop(&mut self) {
        match self.close() {
            Err(err) => panic!("VeracruzServerIceCap::drop failed in call to self.close:{:?}", err),
            _ => (),
        }
    }
}

impl VeracruzServerIceCap {

    fn send(&self, request: &Request) -> Result<Response> {
        let msg = serialize(request).unwrap();
        // log::info!("sending msg {}", msg.len());
        let header = (msg.len() as Header).to_le_bytes();
        let mut stream = self.stream.lock().unwrap();
        stream.write(&header).unwrap();
        stream.write(&msg).unwrap();
        // log::info!("sent, now reading");
        let mut header_bytes = [0; size_of::<Header>()];
        stream.read_exact(&mut header_bytes).unwrap();
        let header = u32::from_le_bytes(header_bytes);
        let mut resp_bytes = vec![0; header as usize];
        stream.read_exact(&mut resp_bytes).unwrap();
        // log::info!("got resp {}", resp_bytes.len());
        let resp = deserialize(&resp_bytes).unwrap();
        Ok(resp)
    }

    fn tls_data_needed(&self, session_id: u32) -> Result<bool> {
        match self.send(&Request::GetTlsDataNeeded(session_id))? {
            Response::GetTlsDataNeeded(needed) => Ok(needed),
            _ => mk_err(UNEXPECTED_RESPONSE),
        }
    }

}
