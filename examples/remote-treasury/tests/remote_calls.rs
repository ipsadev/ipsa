mod router {
    soroban_sdk::contractimport!(file = "../../contracts/stellar_ibc_router.wasm");
}

mod gmp {
    soroban_sdk::contractimport!(file = "../../contracts/stellar_ibc_gmp.wasm");
}

mod gmp_account {
    soroban_sdk::contractimport!(file = "../../contracts/ipsa_gmp_account.wasm");
}

use ipsa_remote_treasury::{IpsaRemoteTreasury, IpsaRemoteTreasuryClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};

const TRUSTED_SENDER: &str = "cosmos1nvhvm94jnpmltlspnemwlgmzf9lemmjm924yw9";

const COUNTERPARTY_CLIENT: &str = "08-wasm-5";

struct Fixture {
    env: Env,
    router: router::Client<'static>,
    gmp: gmp::Client<'static>,
    client_id: String,
}

fn setup() -> Fixture {
    let env = Env::default();

    env.mock_all_auths();
    env.ledger().with_mut(|ledger| ledger.timestamp = 1_000);

    let owner = Address::generate(&env);
    let router_id = env.register(router::WASM, (owner.clone(),));
    let router = router::Client::new(&env, &router_id);
    let light_client = env.register(ipsa_test_light_client::TestLightClient, ());

    router.register_client_type(&String::from_str(&env, "test"), &light_client);

    let client_id = router.create_client(
        &String::from_str(&env, "test"),
        &Bytes::from_slice(&env, b"cs"),
        &Bytes::from_slice(&env, b"cons"),
        &1,
    );

    router.register_counterparty(
        &client_id,
        &String::from_str(&env, COUNTERPARTY_CLIENT),
        &vec![&env, Bytes::from_slice(&env, b"ibc")],
    );

    let gmp_id = env.register(gmp::WASM, (owner, router_id));
    let gmp = gmp::Client::new(&env, &gmp_id);

    gmp.set_account_wasm(&env.deployer().upload_contract_wasm(gmp_account::WASM));
    router.register_port(&String::from_str(&env, "gmpport"), &gmp_id);

    Fixture {
        env,
        router,
        gmp,
        client_id,
    }
}

fn receiver(f: &Fixture) -> IpsaRemoteTreasuryClient<'static> {
    let id = f.env.register(
        IpsaRemoteTreasury,
        (
            f.gmp.address.clone(),
            f.client_id.clone(),
            String::from_str(&f.env, TRUSTED_SENDER),
            Bytes::new(&f.env),
        ),
    );

    IpsaRemoteTreasuryClient::new(&f.env, &id)
}

fn call(f: &Fixture, sender: &str, target: &Address) -> Option<BytesN<32>> {
    let args: Vec<Val> = vec![&f.env, 250u32.into_val(&f.env)];
    let call = ipsa::gmp::GmpCall {
        target: target.clone(),
        function: Symbol::new(&f.env, "set_limit"),
        args,
    };
    let data = gmp::GmpPacketData {
        sender: String::from_str(&f.env, sender),
        receiver: target.to_string(),
        salt: Bytes::new(&f.env),
        payload: call.to_xdr(&f.env),
        memo: String::from_str(&f.env, ""),
    };
    let encoding = String::from_str(&f.env, ipsa::gmp::ENCODING_PROTOBUF);

    f.router.recv_packet(
        &router::Packet {
            sequence: 1,
            source_client: String::from_str(&f.env, COUNTERPARTY_CLIENT),
            dest_client: f.client_id.clone(),
            timeout_timestamp: 2_000,
            payloads: vec![
                &f.env,
                router::Payload {
                    source_port: String::from_str(&f.env, "gmpport"),
                    dest_port: String::from_str(&f.env, "gmpport"),
                    version: String::from_str(&f.env, "ics27-2"),
                    value: f.gmp.encode_packet(&encoding, &data),
                    encoding,
                },
            ],
        },
        &Bytes::from_slice(&f.env, b"proof"),
        &10,
    );

    f.router.acknowledgement(&f.client_id, &1)
}

fn error_acknowledgement_commitment(env: &Env) -> BytesN<32> {
    let mut preimage = Bytes::from_slice(env, &[0x02]);
    let error: Bytes = env
        .crypto()
        .sha256(&Bytes::from_slice(env, b"UNIVERSAL_ERROR_ACKNOWLEDGEMENT"))
        .into();

    preimage.append(&env.crypto().sha256(&error).into());

    env.crypto().sha256(&preimage).into()
}

#[test]
fn the_stored_account_is_the_one_the_gmp_app_derives() {
    let f = setup();
    let receiver = receiver(&f);
    let derived = f.gmp.account_address(
        &f.client_id,
        &String::from_str(&f.env, TRUSTED_SENDER),
        &Bytes::new(&f.env),
    );

    assert_eq!(receiver.remote_account(), derived.address);
}

#[test]
fn a_call_from_the_trusted_sender_runs() {
    let f = setup();
    let receiver = receiver(&f);

    f.env.set_auths(&[]);

    let acknowledgement = call(&f, TRUSTED_SENDER, &receiver.address);

    assert_eq!(receiver.limit(), Some(250));
    assert_ne!(
        acknowledgement,
        Some(error_acknowledgement_commitment(&f.env))
    );
}

#[test]
fn a_call_from_any_other_sender_is_refused() {
    let f = setup();
    let receiver = receiver(&f);

    f.env.set_auths(&[]);

    let acknowledgement = call(&f, "cosmos1stranger", &receiver.address);

    assert_eq!(receiver.limit(), None);
    assert_eq!(
        acknowledgement,
        Some(error_acknowledgement_commitment(&f.env))
    );
}
