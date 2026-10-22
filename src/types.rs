use soroban_sdk::{contracttype, Bytes, Env, String, Vec};

pub const UNIVERSAL_ERROR_ACKNOWLEDGEMENT: &[u8] = b"UNIVERSAL_ERROR_ACKNOWLEDGEMENT";

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payload {
    pub source_port: String,
    pub dest_port: String,
    pub version: String,
    pub encoding: String,
    pub value: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Packet {
    pub sequence: u64,
    pub source_client: String,
    pub dest_client: String,
    pub timeout_timestamp: u64,
    pub payloads: Vec<Payload>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counterparty {
    pub client_id: String,
    pub commitment_prefix: Vec<Bytes>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnRecvPacketCallback {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
    pub payload: Payload,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnAcknowledgementPacketCallback {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
    pub payload: Payload,
    pub acknowledgement: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnTimeoutPacketCallback {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
    pub payload: Payload,
}

pub fn error_acknowledgement(env: &Env) -> Bytes {
    let hash = env
        .crypto()
        .sha256(&Bytes::from_slice(env, UNIVERSAL_ERROR_ACKNOWLEDGEMENT))
        .to_array();

    Bytes::from_array(env, &hash)
}

pub fn is_error_acknowledgement(env: &Env, acknowledgement: &Bytes) -> bool {
    *acknowledgement == error_acknowledgement(env)
}
