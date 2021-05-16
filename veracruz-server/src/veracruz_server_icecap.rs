use std::{
    env,
    fs::{OpenOptions, File},
    io::{Read, Write},
    mem::size_of,
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    result,
    sync::Mutex,
    string::ToString,
    process::Command,
};
use bincode::{serialize, deserialize};
use veracruz_utils::platform::icecap::message::{Request, Response, Header};
use crate::veracruz_server::{VeracruzServer, VeracruzServerError};

const ENDPOINT_ENV: &str = "VERACRUZ_SERVER_ENDPOINT";

const UNEXPECTED_RESPONSE: &str = "unexpected response";
const NOT_IMPLEMENTED: &str = "not implemented";

type Result<T> = result::Result<T, VeracruzServerError>;

fn mk_err<T>(msg: impl ToString) -> Result<T> {
    Err(VeracruzServerError::DirectStringError(msg.to_string()))
}

fn mk_err_raw(msg: impl ToString) -> VeracruzServerError {
    VeracruzServerError::DirectStringError(msg.to_string())
}

fn get_endpoint() -> Result<File> {
    let path = env::var(ENDPOINT_ENV).map_err(mk_err_raw)?;
    OpenOptions::new().read(true).write(true).open(path).map_err(mk_err_raw)
}

pub struct VeracruzServerIceCap {
    stream: Mutex<File>,
}

impl VeracruzServer for VeracruzServerIceCap {

    fn new(policy_json: &str) -> Result<Self> {
        destroy_realm(); // HACK
        create_realm();
        run_realm();

        let server = Self {
            stream: Mutex::new(get_endpoint()?),
        };
        match server.send(&Request::New { policy_json: policy_json.to_string() })? {
            Response::New => {
                Ok(server)
            }
            _ => {
                mk_err(UNEXPECTED_RESPONSE)
            }
        }
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
        destroy_realm();
        Ok(true)
    }
}

impl Drop for VeracruzServerIceCap {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            panic!("Veracruz server failed to close: {}", err)
        }
    }
}

impl VeracruzServerIceCap {

    fn send(&self, request: &Request) -> Result<Response> {
        let msg = serialize(request).unwrap();
        let header = (msg.len() as Header).to_le_bytes();
        let mut stream = self.stream.lock().unwrap();
        stream.write(&header).unwrap();
        stream.write(&msg).unwrap();
        let mut header_bytes = [0; size_of::<Header>()];
        stream.read_exact(&mut header_bytes).unwrap();
        let header = u32::from_le_bytes(header_bytes);
        let mut resp_bytes = vec![0; header as usize];
        stream.read_exact(&mut resp_bytes).unwrap();
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

fn create_realm() {
    let status = Command::new("icecap-host")
        .arg("create")
        .arg("0")
        .arg("/spec.bin")
        .arg("file:/dev/rb_resource_server")
        .status().unwrap();
    assert!(status.success());
}

fn run_realm() {
    let status = Command::new("icecap-host")
        .arg("hack-run")
        .arg("0")
        .status().unwrap();
    assert!(status.success());
}

fn destroy_realm() {
    let status = Command::new("icecap-host")
        .arg("destroy")
        .arg("0")
        .status().unwrap();
    assert!(status.success());
}
