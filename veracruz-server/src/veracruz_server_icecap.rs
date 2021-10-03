//! IceCap-specific material for the Veracruz server
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.markdown` file in the Veracruz root directory for
//! information on licensing and copyright.

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
    process::{Command, Child},
};
use err_derive::Error;
use bincode::{serialize, deserialize};
use veracruz_utils::{
    policy::policy::Policy,
    platform::icecap::message::{Request, Response, Header},
};
use crate::veracruz_server::{VeracruzServer, VeracruzServerError};

const VERACRUZ_ICECAP_HOST_COMMAND_ENV: &str = "VERACRUZ_ICECAP_HOST_COMMAND";
const VERACRUZ_ICECAP_REALM_ID_ENV: &str = "VERACRUZ_ICECAP_REALM_ID";
const VERACRUZ_ICECAP_REALM_SPEC_ENV: &str = "VERACRUZ_ICECAP_REALM_SPEC";
const VERACRUZ_ICECAP_REALM_ENDPOINT_ENV: &str = "VERACRUZ_ICECAP_REALM_ENDPOINT";

const VERACRUZ_ICECAP_HOST_COMMAND_DEFAULT: &str = "icecap-host";

type Result<T> = result::Result<T, VeracruzServerError>;

#[derive(Debug, Error)]
pub enum IceCapError {
    #[error(display = "IceCap: Realm channel error")]
    RealmChannelError,
    #[error(display = "IceCap: Unexpected response from runtime manager: {:?}", _0)]
    UnexpectedRuntimeManagerResponse(Response),
    #[error(display = "IceCap: Missing environment variable: {}", variable)]
    MissingEnvironmentVariable { variable: String },
    #[error(display = "IceCap: Invalid environment variable value: {}", variable)]
    InvalidEnvironemntVariableValue { variable: String },
}

struct Configuration {
    icecap_host_command: PathBuf,
    realm_id: usize,
    realm_spec: PathBuf,
    realm_endpoint: PathBuf,
}

impl Configuration {

    fn env_var(var: &str) -> Result<String> {
        env::var(var).map_err(|_| VeracruzServerError::IceCapError(IceCapError::MissingEnvironmentVariable { variable: var.to_string() }))
    }

    fn from_env() -> Result<Self> {
        Ok(Self {
            icecap_host_command: Self::env_var(VERACRUZ_ICECAP_HOST_COMMAND_ENV).map(PathBuf::from).unwrap_or(VERACRUZ_ICECAP_HOST_COMMAND_DEFAULT.into()),
            realm_id: Self::env_var(VERACRUZ_ICECAP_REALM_ID_ENV)?.parse::<usize>().map_err(|_|
                VeracruzServerError::IceCapError(IceCapError::InvalidEnvironemntVariableValue { variable: VERACRUZ_ICECAP_REALM_ID_ENV.to_string() })
            )?,
            realm_spec: Self::env_var(VERACRUZ_ICECAP_REALM_SPEC_ENV)?.into(),
            realm_endpoint: Self::env_var(VERACRUZ_ICECAP_REALM_ENDPOINT_ENV)?.into(),
        })
    }

    fn hack_command() -> Command {
        let mut command = Command::new("taskset");
        command.arg("0x2");
        command
    }

    fn hack_ensure_not_realm_running() {
        Command::new("pkill").arg("icecap-host").status().unwrap();
    }

    fn create_realm(&self) -> Result<()> {
        let status = Self::hack_command()
            .arg(&self.icecap_host_command)
            .arg("create")
            .arg(format!("{}", self.realm_id))
            .arg(&self.realm_spec)
            .status().unwrap();
        assert!(status.success());
        Ok(())
    }

    fn run_realm(&self) -> Result<Child> {
        let virtual_node_id: usize = 0;
        let child = Self::hack_command()
            .arg(&self.icecap_host_command)
            .arg("run")
            .arg(format!("{}", self.realm_id))
            .arg(format!("{}", virtual_node_id))
            .spawn().unwrap();
        Ok(child)
    }

    fn destroy_realm(&self) -> Result<()> {
        // HACK clean up in case of previous failure
        Self::hack_ensure_not_realm_running();

        let status = Self::hack_command()
            .arg(&self.icecap_host_command)
            .arg("destroy")
            .arg(format!("{}", self.realm_id))
            .status().unwrap();
        assert!(status.success());
        Ok(())
    }

}

pub struct VeracruzServerIceCap {
    configuration: Configuration,
    realm_channel: Mutex<File>,
    realm_process: Child,

    // HACK
    device_id: i32,
}

impl VeracruzServer for VeracruzServerIceCap {

    fn new(policy_json: &str) -> Result<Self> {

        let policy: Policy = Policy::from_json(policy_json)?;

        let device_id = hack::native_attestation(&policy.proxy_attestation_server_url())?;

        let configuration = Configuration::from_env()?;
        configuration.destroy_realm()?; // HACK
        configuration.create_realm()?;
        let realm_process = configuration.run_realm()?;
        let realm_channel = Mutex::new(
            OpenOptions::new().read(true).write(true).open(&configuration.realm_endpoint)
                .map_err(|_| VeracruzServerError::IceCapError(IceCapError::RealmChannelError))?
        );
        let server = Self {
            configuration,
            realm_channel,
            realm_process,
            device_id,
        };
        match server.send(&Request::New { policy_json: policy_json.to_string() })? {
            Response::New => (),
            resp => return Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
        }
        Ok(server)
    }

    fn proxy_psa_attestation_get_token(
        &mut self,
        challenge: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<u8>, i32)> {
        let enclave_cert = self.get_enclave_cert()?;
        let (token, device_public_key) = hack::proxy_attesation(&challenge, &enclave_cert)?;
        Ok((token, device_public_key, self.device_id))
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
           resp => Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
       }
    }

    fn get_enclave_name(&mut self) -> Result<String> {
       match self.send(&Request::GetEnclaveName)? {
           Response::GetEnclaveName(name) => Ok(name),
           resp => Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
       }
    }

    fn new_tls_session(&mut self) -> Result<u32> {
       match self.send(&Request::NewTlsSession)? {
           Response::NewTlsSession(session_id) => Ok(session_id),
           resp => Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
       }
    }

    fn close_tls_session(&mut self, session_id: u32) -> Result<()> {
       match self.send(&Request::CloseTlsSession(session_id))? {
           Response::CloseTlsSession => Ok(()),
           resp => Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
       }
    }

    fn tls_data(&mut self, session_id: u32, input: Vec<u8>) -> Result<(bool, Option<Vec<Vec<u8>>>)> {
        match self.send(&Request::SendTlsData(session_id, input))? {
            Response::SendTlsData => (),
            resp => return Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
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
                resp => return Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
            };
        };

        Ok((active, match acc.len() {
            0 => None,
            _ => Some(acc),
        }))
    }

    fn close(&mut self) -> Result<bool> {
        self.realm_process.kill().unwrap();
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
        let mut realm_channel = self.realm_channel.lock().unwrap();
        realm_channel.write(&header).unwrap();
        realm_channel.write(&msg).unwrap();
        let mut header_bytes = [0; size_of::<Header>()];
        realm_channel.read_exact(&mut header_bytes).unwrap();
        let header = u32::from_le_bytes(header_bytes);
        let mut resp_bytes = vec![0; header as usize];
        realm_channel.read_exact(&mut resp_bytes).unwrap();
        let resp = deserialize(&resp_bytes).unwrap();
        Ok(resp)
    }

    fn tls_data_needed(&self, session_id: u32) -> Result<bool> {
        match self.send(&Request::GetTlsDataNeeded(session_id))? {
            Response::GetTlsDataNeeded(needed) => Ok(needed),
            resp => Err(VeracruzServerError::IceCapError(IceCapError::UnexpectedRuntimeManagerResponse(resp))),
        }
    }

}

