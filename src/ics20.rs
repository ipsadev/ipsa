use soroban_sdk::{contractclient, contracttype, Address, Bytes, Env, String, Vec};

pub const PORT: &str = "transfer";

pub const VERSION: &str = "ics20-1";

pub const ENCODING_JSON: &str = "application/json";

pub const ENCODING_SOLIDITY_ABI: &str = "application/x-solidity-abi";

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetKind {
    Local,
    Voucher,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetInfo {
    pub address: Address,
    pub kind: AssetKind,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetEntry {
    pub denom: String,
    pub address: Address,
    pub kind: AssetKind,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub denom: String,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FungibleTokenPacketData {
    pub token: Token,
    pub sender: String,
    pub receiver: String,
    pub memo: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferReceived {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
    pub sender: String,
    pub token: Address,
    pub amount: i128,
    pub memo: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferAcknowledged {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
    pub success: bool,
    pub acknowledgement: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferTimedOut {
    pub source_client: String,
    pub dest_client: String,
    pub sequence: u64,
}

#[contractclient(name = "TransferClient")]
pub trait Transfer {
    fn initiate_transfer(
        env: Env,
        sender: Address,
        source_client_id: String,
        denom: String,
        amount: i128,
        receiver: String,
        timeout_timestamp: u64,
        memo: String,
    ) -> u64;

    fn asset(env: Env, denom: String) -> AssetInfo;

    fn assets(env: Env) -> Vec<AssetEntry>;

    fn balance_of(env: Env, who: Address, denom: String) -> i128;

    fn client_encoding(env: Env, client_id: String) -> String;
}

pub trait TransferSender {
    fn on_transfer_acknowledgement(env: Env, transfer: Address, result: TransferAcknowledged);

    fn on_transfer_timeout(env: Env, transfer: Address, result: TransferTimedOut);
}

pub trait TransferReceiver {
    fn on_ibc_transfer_received(env: Env, transfer: Address, received: TransferReceived);
}
