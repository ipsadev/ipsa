#![no_std]

pub use ipsa::gmp::{GmpAcknowledged, GmpTimedOut};
use ipsa::gmp::{GmpCaller, IbcGmpClient, ENCODING_SOLIDITY_ABI};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Bytes, Env,
    String,
};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_macros::only_owner;

pub const ACTION_TIMEOUT_SECONDS: u64 = 6 * 60 * 60;

const EVM_ADDRESS_LENGTH: usize = 42;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    UntrustedGmpApp = 1,
    InvalidEvmAddress = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Freeze(String),
    Unfreeze(String),
    Pause,
    Unpause,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pending,
    Executed(Bytes),
    Rejected,
    TimedOut,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub action: Action,
    pub outcome: Outcome,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub gmp: Address,
    pub client_id: String,
    pub token: String,
}

#[contracttype]
enum DataKey {
    Config,
    Record(u64),
}

#[contract]
pub struct IpsaIssuerConsole;

#[contractimpl(contracttrait)]
impl Ownable for IpsaIssuerConsole {}

#[contractimpl]
impl IpsaIssuerConsole {
    pub fn __constructor(env: Env, owner: Address, gmp: Address, client_id: String, token: String) {
        evm_address(&env, &token);
        set_owner(&env, &owner);
        env.storage().instance().set(
            &DataKey::Config,
            &Config {
                gmp,
                client_id,
                token,
            },
        );
    }

    #[only_owner]
    pub fn freeze(env: Env, account: String) -> u64 {
        dispatch(&env, Action::Freeze(account))
    }

    #[only_owner]
    pub fn unfreeze(env: Env, account: String) -> u64 {
        dispatch(&env, Action::Unfreeze(account))
    }

    #[only_owner]
    pub fn pause(env: Env) -> u64 {
        dispatch(&env, Action::Pause)
    }

    #[only_owner]
    pub fn unpause(env: Env) -> u64 {
        dispatch(&env, Action::Unpause)
    }

    #[only_owner]
    pub fn set_token(env: Env, token: String) {
        evm_address(&env, &token);

        let mut config = config(&env);
        config.token = token;
        env.storage().instance().set(&DataKey::Config, &config);
    }

    pub fn record(env: Env, sequence: u64) -> Option<Record> {
        env.storage().persistent().get(&DataKey::Record(sequence))
    }

    pub fn config(env: Env) -> Config {
        config(&env)
    }
}

#[contractimpl]
impl GmpCaller for IpsaIssuerConsole {
    fn on_gmp_acknowledgement(env: Env, gmp: Address, result: GmpAcknowledged) {
        trusted(&env, &gmp);

        let outcome = if result.success {
            Outcome::Executed(result.result)
        } else {
            Outcome::Rejected
        };

        settle(&env, result.sequence, outcome);
    }

    fn on_gmp_timeout(env: Env, gmp: Address, result: GmpTimedOut) {
        trusted(&env, &gmp);

        settle(&env, result.sequence, Outcome::TimedOut);
    }
}

fn config(env: &Env) -> Config {
    env.storage().instance().get(&DataKey::Config).unwrap()
}

fn dispatch(env: &Env, action: Action) -> u64 {
    let config = config(env);
    let payload = calldata(env, &action);

    let sequence = IbcGmpClient::new(env, &config.gmp).send_call(
        &env.current_contract_address(),
        &config.client_id,
        &config.token,
        &Bytes::new(env),
        &payload,
        &(env.ledger().timestamp() + ACTION_TIMEOUT_SECONDS),
        &String::from_str(env, ""),
        &String::from_str(env, ENCODING_SOLIDITY_ABI),
    );

    env.storage().persistent().set(
        &DataKey::Record(sequence),
        &Record {
            action,
            outcome: Outcome::Pending,
        },
    );

    sequence
}

fn settle(env: &Env, sequence: u64, outcome: Outcome) {
    let key = DataKey::Record(sequence);

    if let Some(mut record) = env.storage().persistent().get::<_, Record>(&key) {
        record.outcome = outcome;
        env.storage().persistent().set(&key, &record);
    }
}

fn trusted(env: &Env, gmp: &Address) {
    if *gmp != config(env).gmp {
        panic_with_error!(env, Error::UntrustedGmpApp);
    }

    gmp.require_auth();
}

pub fn calldata(env: &Env, action: &Action) -> Bytes {
    let (signature, account) = match action {
        Action::Freeze(account) => ("freeze(address)", Some(account)),
        Action::Unfreeze(account) => ("unfreeze(address)", Some(account)),
        Action::Pause => ("pause()", None),
        Action::Unpause => ("unpause()", None),
    };

    let selector = env
        .crypto()
        .keccak256(&Bytes::from_slice(env, signature.as_bytes()))
        .to_array();
    let mut out = Bytes::from_slice(env, &selector[..4]);

    if let Some(account) = account {
        let mut word = [0u8; 32];
        word[12..].copy_from_slice(&evm_address(env, account));
        out.append(&Bytes::from_array(env, &word));
    }

    out
}

fn evm_address(env: &Env, text: &String) -> [u8; 20] {
    if text.len() as usize != EVM_ADDRESS_LENGTH {
        panic_with_error!(env, Error::InvalidEvmAddress);
    }

    let mut ascii = [0u8; EVM_ADDRESS_LENGTH];
    text.copy_into_slice(&mut ascii);

    if &ascii[..2] != b"0x" {
        panic_with_error!(env, Error::InvalidEvmAddress);
    }

    let mut address = [0u8; 20];

    for (index, pair) in ascii[2..].chunks(2).enumerate() {
        address[index] = (nibble(env, pair[0]) << 4) | nibble(env, pair[1]);
    }

    address
}

fn nibble(env: &Env, character: u8) -> u8 {
    match character {
        b'0'..=b'9' => character - b'0',
        b'a'..=b'f' => character - b'a' + 10,
        b'A'..=b'F' => character - b'A' + 10,
        _ => panic_with_error!(env, Error::InvalidEvmAddress),
    }
}
