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

const RESOURCE_SERVER_ENDPOINT_ENV: &str = "VERACRUZ_RESOURCE_SERVER_ENDPOINT";
const REALM_ENDPOINT_ENV: &str = "VERACRUZ_REALM_ENDPOINT";
const REALM_ID_ENV: &str = "VERACRUZ_REALM_ID";
const REALM_SPEC_ENV: &str = "VERACRUZ_REALM_SPEC";

type Result<T> = result::Result<T, VeracruzServerError>;

struct Configuration {
    realm_id: usize,
    realm_spec: PathBuf,
    realm_endpoint: PathBuf,
    resource_server_endpoint: PathBuf,
}

impl Configuration {

    fn env_var(var: &str) -> Result<String> {
        env::var(var).map_err(lame_err)
    }

    fn from_env() -> Result<Self> {
        Ok(Self {
            realm_id: Self::env_var(REALM_ID_ENV)?.parse::<usize>().map_err(lame_err)?,
            realm_spec: Self::env_var(REALM_SPEC_ENV)?.into(),
            realm_endpoint: Self::env_var(REALM_ENDPOINT_ENV)?.into(),
            resource_server_endpoint: Self::env_var(RESOURCE_SERVER_ENDPOINT_ENV)?.into(),
        })
    }

    fn create_realm(&self) -> Result<()> {
        let status = Command::new("icecap-host")
            .arg("create")
            .arg(format!("{}", self.realm_id))
            .arg(&self.realm_spec)
            .arg(&self.resource_server_endpoint)
            .status().unwrap();
        assert!(status.success());
        Ok(())
    }
    
    fn run_realm(&self) -> Result<()> {
        let status = Command::new("icecap-host")
            .arg("hack-run")
            .arg(format!("{}", self.realm_id))
            .status().unwrap();
        assert!(status.success());
        Ok(())
    }
    
    fn destroy_realm(&self) -> Result<()> {
        let status = Command::new("icecap-host")
            .arg("destroy")
            .arg(format!("{}", self.realm_id))
            .status().unwrap();
        assert!(status.success());
        Ok(())
    }
    
}

pub struct VeracruzServerIceCap {
    configuration: Configuration,
    realm_handle: Mutex<File>,
}

impl VeracruzServer for VeracruzServerIceCap {

    fn new(policy_json: &str) -> Result<Self> {
        let configuration = Configuration::from_env()?;
        configuration.destroy_realm()?; // HACK
        configuration.create_realm()?;
        configuration.run_realm()?;
        let realm_handle = Mutex::new(
            OpenOptions::new().read(true).write(true).open(&configuration.realm_endpoint)
                .map_err(lame_err)?
        );
        let server = Self {
            configuration,
            realm_handle,
        };
        match server.send(&Request::New { policy_json: policy_json.to_string() })? {
            Response::New => (),
            _ => return Err(unexpected_response()),
        }
        Ok(server)
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
           _ => Err(unexpected_response()),
       }
    }

    fn get_enclave_name(&mut self) -> Result<String> {
       match self.send(&Request::GetEnclaveName)? {
           Response::GetEnclaveName(name) => Ok(name),
           _ => Err(unexpected_response()),
       }
    }

    fn new_tls_session(&mut self) -> Result<u32> {
       match self.send(&Request::NewTlsSession)? {
           Response::NewTlsSession(session_id) => Ok(session_id),
           _ => Err(unexpected_response()),
       }
    }

    fn close_tls_session(&mut self, session_id: u32) -> Result<()> {
       match self.send(&Request::CloseTlsSession(session_id))? {
           Response::CloseTlsSession => Ok(()),
           _ => Err(unexpected_response()),
       }
    }

    fn tls_data(&mut self, session_id: u32, input: Vec<u8>) -> Result<(bool, Option<Vec<Vec<u8>>>)> {
        match self.send(&Request::SendTlsData(session_id, input))? {
            Response::SendTlsData => (),
            _ => return Err(unexpected_response()),
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
                _ => return Err(unexpected_response()),
            };
        };

        Ok((active, match acc.len() {
            0 => None,
            _ => Some(acc),
        }))
    }

    fn close(&mut self) -> Result<bool> {
        self.configuration.destroy_realm()?;
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
        let mut realm_handle = self.realm_handle.lock().unwrap();
        realm_handle.write(&header).unwrap();
        realm_handle.write(&msg).unwrap();
        let mut header_bytes = [0; size_of::<Header>()];
        realm_handle.read_exact(&mut header_bytes).unwrap();
        let header = u32::from_le_bytes(header_bytes);
        let mut resp_bytes = vec![0; header as usize];
        realm_handle.read_exact(&mut resp_bytes).unwrap();
        let resp = deserialize(&resp_bytes).unwrap();
        Ok(resp)
    }

    fn tls_data_needed(&self, session_id: u32) -> Result<bool> {
        match self.send(&Request::GetTlsDataNeeded(session_id))? {
            Response::GetTlsDataNeeded(needed) => Ok(needed),
            _ => Err(unexpected_response()),
        }
    }

}

fn unexpected_response() -> VeracruzServerError {
    // HACK
    VeracruzServerError::DirectStringError("unexpected response".to_string())
}

// HACK
fn lame_err(msg: impl ToString) -> VeracruzServerError {
    VeracruzServerError::DirectStringError(msg.to_string())
}
