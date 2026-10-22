#![no_std]

use ipsa::ics20::{TransferAcknowledged, TransferClient, TransferSender, TransferTimedOut};
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contracterror, contractimpl, contracttype, panic_with_error, vec, Address, Env,
    IntoVal, String, Symbol,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    UntrustedTransferApp = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pending,
    Acknowledged(TransferAcknowledged),
    TimedOut(TransferTimedOut),
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Owner,
    Transfer,
    Outcome(String, u64),
}

#[contract]
pub struct IpsaPayouts;

#[contractimpl]
impl IpsaPayouts {
    pub fn __constructor(env: Env, owner: Address, transfer: Address) {
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::Transfer, &transfer);
    }

    pub fn send(
        env: Env,
        source_client_id: String,
        denom: String,
        amount: i128,
        receiver: String,
        timeout_timestamp: u64,
    ) -> u64 {
        let owner: Address = env.storage().instance().get(&DataKey::Owner).unwrap();

        owner.require_auth();

        let transfer = transfer_app(&env);
        let client = TransferClient::new(&env, &transfer);
        let token = client.asset(&denom).address;

        authorize_escrow(&env, &token, &transfer, amount);

        let sequence = client.initiate_transfer(
            &env.current_contract_address(),
            &source_client_id,
            &denom,
            &amount,
            &receiver,
            &timeout_timestamp,
            &String::from_str(&env, ""),
        );

        env.storage().persistent().set(
            &DataKey::Outcome(source_client_id, sequence),
            &Outcome::Pending,
        );

        sequence
    }

    pub fn outcome(env: Env, source_client_id: String, sequence: u64) -> Option<Outcome> {
        env.storage()
            .persistent()
            .get(&DataKey::Outcome(source_client_id, sequence))
    }

    pub fn transfer(env: Env) -> Address {
        transfer_app(&env)
    }
}

#[contractimpl]
impl TransferSender for IpsaPayouts {
    fn on_transfer_acknowledgement(env: Env, transfer: Address, result: TransferAcknowledged) {
        trusted(&env, &transfer);

        env.storage().persistent().set(
            &DataKey::Outcome(result.source_client.clone(), result.sequence),
            &Outcome::Acknowledged(result),
        );
    }

    fn on_transfer_timeout(env: Env, transfer: Address, result: TransferTimedOut) {
        trusted(&env, &transfer);

        env.storage().persistent().set(
            &DataKey::Outcome(result.source_client.clone(), result.sequence),
            &Outcome::TimedOut(result),
        );
    }
}

fn transfer_app(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Transfer).unwrap()
}

fn trusted(env: &Env, transfer: &Address) {
    if *transfer != transfer_app(env) {
        panic_with_error!(env, Error::UntrustedTransferApp);
    }

    transfer.require_auth();
}

fn authorize_escrow(env: &Env, token: &Address, transfer: &Address, amount: i128) {
    let escrow = SubContractInvocation {
        context: ContractContext {
            contract: token.clone(),
            fn_name: Symbol::new(env, "transfer"),
            args: (env.current_contract_address(), transfer.clone(), amount).into_val(env),
        },
        sub_invocations: vec![env],
    };

    env.authorize_as_current_contract(vec![env, InvokerContractAuthEntry::Contract(escrow)]);
}
