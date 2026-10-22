use soroban_sdk::{
    contractclient, contracttype, xdr::ToXdr, Address, Bytes, BytesN, Env, String, Symbol, Val, Vec,
};

pub const PORT: &str = "gmpport";

pub const VERSION: &str = "ics27-2";

pub const ENCODING_SOROBAN_XDR: &str = "application/x-soroban-xdr";

pub const ENCODING_JSON: &str = "application/json";

pub const ENCODING_PROTOBUF: &str = "application/x-protobuf";

pub const ENCODING_SOLIDITY_ABI: &str = "application/x-solidity-abi";

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GmpPacketData {
    pub sender: String,
    pub receiver: String,
    pub salt: Bytes,
    pub payload: Bytes,
    pub memo: String,
}

#[contracttype]
#[derive(Clone)]
pub struct GmpCall {
    pub target: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Acknowledgement {
    pub result: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedAccount {
    pub address: Address,
    pub materialized: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GmpAcknowledged {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
    pub success: bool,
    pub result: Bytes,
    pub acknowledgement: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GmpTimedOut {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
}

#[contractclient(name = "IbcGmpClient")]
pub trait IbcGmp {
    fn send_call(
        env: Env,
        sender: Address,
        source_client_id: String,
        receiver: String,
        salt: Bytes,
        payload: Bytes,
        timeout_timestamp: u64,
        memo: String,
        encoding: String,
    ) -> u64;

    fn account_address(env: Env, client_id: String, sender: String, salt: Bytes) -> DerivedAccount;

    fn encode_packet(env: Env, encoding: String, packet: GmpPacketData) -> Bytes;

    fn decode_packet(env: Env, encoding: String, value: Bytes) -> GmpPacketData;
}

pub trait GmpCaller {
    fn on_gmp_acknowledgement(env: Env, gmp: Address, result: GmpAcknowledged);

    fn on_gmp_timeout(env: Env, gmp: Address, result: GmpTimedOut);
}

pub fn account_salt(env: &Env, client_id: &String, sender: &String, salt: &Bytes) -> BytesN<32> {
    let mut preimage = Bytes::new(env);

    append_length_prefixed(env, &mut preimage, &client_id.clone().to_xdr(env));
    append_length_prefixed(env, &mut preimage, &sender.clone().to_xdr(env));
    append_length_prefixed(env, &mut preimage, salt);

    env.crypto().sha256(&preimage).into()
}

pub fn remote_account(
    env: &Env,
    gmp: &Address,
    client_id: &String,
    sender: &String,
    salt: &Bytes,
) -> Address {
    env.deployer()
        .with_address(gmp.clone(), account_salt(env, client_id, sender, salt))
        .deployed_address()
}

fn append_length_prefixed(env: &Env, out: &mut Bytes, part: &Bytes) {
    out.append(&Bytes::from_array(env, &part.len().to_be_bytes()));
    out.append(part);
}
