use serde::{Serialize, Deserialize};

pub type Header = u32;
pub type SessionId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    New { policy_json: String },
    GetEnclaveCert,
    GetEnclaveName,
    NewTlsSession,
    CloseTlsSession(SessionId),
    SendTlsData(SessionId, Vec<u8>),
    GetTlsDataNeeded(SessionId),
    GetTlsData(SessionId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    New,
    GetEnclaveCert(Vec<u8>),
    GetEnclaveName(String),
    NewTlsSession(SessionId),
    CloseTlsSession,
    SendTlsData,
    GetTlsDataNeeded(bool),
    GetTlsData(bool, Vec<u8>),
    Error(Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    Unspecified,
}
