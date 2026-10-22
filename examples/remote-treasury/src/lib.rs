#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, String};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    RemoteAccount,
    Limit,
}

#[contract]
pub struct IpsaRemoteTreasury;

#[contractimpl]
impl IpsaRemoteTreasury {
    pub fn __constructor(env: Env, gmp: Address, client_id: String, sender: String, salt: Bytes) {
        let remote = ipsa::gmp::remote_account(&env, &gmp, &client_id, &sender, &salt);

        env.storage()
            .instance()
            .set(&DataKey::RemoteAccount, &remote);
    }

    pub fn set_limit(env: Env, limit: u32) -> u32 {
        remote_account(&env).require_auth();

        env.storage().instance().set(&DataKey::Limit, &limit);

        limit
    }

    pub fn limit(env: Env) -> Option<u32> {
        env.storage().instance().get(&DataKey::Limit)
    }

    pub fn remote_account(env: Env) -> Address {
        remote_account(&env)
    }
}

fn remote_account(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::RemoteAccount)
        .unwrap()
}
