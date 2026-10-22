mod router {
    soroban_sdk::contractimport!(file = "../../contracts/stellar_ibc_router.wasm");
}

mod transfer {
    soroban_sdk::contractimport!(file = "../../contracts/stellar_ibc_transfer.wasm");
}

use ipsa::ics20::{TransferAcknowledged, TransferTimedOut};
use ipsa_payouts::{Error, IpsaPayouts, IpsaPayoutsClient, Outcome};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Bytes, Env, String,
};

const RECEIVER: &str = "cosmos1nvhvm94jnpmltlspnemwlgmzf9lemmjm924yw9";

struct Fixture {
    env: Env,
    router: router::Client<'static>,
    transfer: transfer::Client<'static>,
    caller: IpsaPayoutsClient<'static>,
    token: TokenClient<'static>,
    client_id: String,
    counterparty_id: String,
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
    let counterparty_id = String::from_str(&env, "08-wasm-5");

    router.register_counterparty(
        &client_id,
        &counterparty_id,
        &vec![&env, Bytes::from_slice(&env, b"ibc")],
    );

    let transfer_id = env.register(transfer::WASM, (router_id.clone(), owner.clone()));
    let transfer = transfer::Client::new(&env, &transfer_id);
    let token_id = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();

    router.register_port(&String::from_str(&env, "transfer"), &transfer_id);
    transfer.register_asset(&String::from_str(&env, "XLM"), &token_id, &false);

    let caller_id = env.register(IpsaPayouts, (owner, transfer_id));

    StellarAssetClient::new(&env, &token_id).mint(&caller_id, &1_000);

    Fixture {
        router,
        transfer,
        caller: IpsaPayoutsClient::new(&env, &caller_id),
        token: TokenClient::new(&env, &token_id),
        client_id,
        counterparty_id,
        env,
    }
}

fn payload(f: &Fixture) -> router::Payload {
    let value = format!(
        "{{\"denom\":\"XLM\",\"amount\":\"400\",\"sender\":\"{}\",\"receiver\":\"{RECEIVER}\",\"memo\":\"\"}}",
        f.caller.address.to_string()
    );

    router::Payload {
        source_port: String::from_str(&f.env, "transfer"),
        dest_port: String::from_str(&f.env, "transfer"),
        version: String::from_str(&f.env, "ics20-1"),
        encoding: String::from_str(&f.env, "application/json"),
        value: Bytes::from_slice(&f.env, value.as_bytes()),
    }
}

fn send(f: &Fixture) -> u64 {
    f.caller.send(
        &f.client_id,
        &String::from_str(&f.env, "XLM"),
        &400,
        &String::from_str(&f.env, RECEIVER),
        &2_000,
    )
}

fn acknowledge(f: &Fixture, sequence: u64, acknowledgement: &[u8]) {
    f.router.acknowledge_packet(
        &router::Packet {
            sequence,
            source_client: f.client_id.clone(),
            dest_client: f.counterparty_id.clone(),
            timeout_timestamp: 2_000,
            payloads: vec![&f.env, payload(f)],
        },
        &vec![&f.env, Bytes::from_slice(&f.env, acknowledgement)],
        &Bytes::from_slice(&f.env, b"proof"),
        &10,
    );
}

#[test]
fn a_sent_transfer_escrows_the_tokens_and_is_pending() {
    let f = setup();
    let sequence = send(&f);

    assert_eq!(sequence, 1);
    assert_eq!(f.token.balance(&f.caller.address), 600);
    assert_eq!(f.token.balance(&f.transfer.address), 400);
    assert_eq!(
        f.caller.outcome(&f.client_id, &sequence),
        Some(Outcome::Pending)
    );
}

#[test]
fn a_delivered_transfer_is_recorded_as_a_success() {
    let f = setup();
    let sequence = send(&f);

    acknowledge(&f, sequence, b"{\"result\":\"AQ==\"}");

    let Some(Outcome::Acknowledged(acknowledged)) = f.caller.outcome(&f.client_id, &sequence)
    else {
        panic!("the transfer was not acknowledged");
    };

    assert!(acknowledged.success);
    assert_eq!(f.token.balance(&f.caller.address), 600);
}

#[test]
fn a_rejected_transfer_is_refunded_and_recorded_as_a_failure() {
    let f = setup();
    let sequence = send(&f);

    acknowledge(&f, sequence, b"\xff\xff");

    let Some(Outcome::Acknowledged(acknowledged)) = f.caller.outcome(&f.client_id, &sequence)
    else {
        panic!("the transfer was not acknowledged");
    };

    assert!(!acknowledged.success);
    assert_eq!(f.token.balance(&f.caller.address), 1_000);
}

#[test]
fn a_timed_out_transfer_is_refunded_and_recorded_as_timed_out() {
    let f = setup();
    let sequence = send(&f);
    let payload = payload(&f);

    f.transfer
        .on_timeout_packet(&transfer::OnTimeoutPacketCallback {
            source_client: f.client_id.clone(),
            dest_client: f.counterparty_id.clone(),
            sequence,
            payload: transfer::Payload {
                source_port: payload.source_port,
                dest_port: payload.dest_port,
                version: payload.version,
                encoding: payload.encoding,
                value: payload.value,
            },
        });

    assert!(matches!(
        f.caller.outcome(&f.client_id, &sequence),
        Some(Outcome::TimedOut(_))
    ));
    assert_eq!(f.token.balance(&f.caller.address), 1_000);
}

#[test]
fn a_callback_from_another_contract_is_refused() {
    let f = setup();
    let impostor = Address::generate(&f.env);
    let acknowledged = TransferAcknowledged {
        source_client: f.client_id.clone(),
        dest_client: f.counterparty_id.clone(),
        sequence: 1,
        success: true,
        acknowledgement: Bytes::new(&f.env),
    };
    let timed_out = TransferTimedOut {
        source_client: f.client_id.clone(),
        dest_client: f.counterparty_id.clone(),
        sequence: 1,
    };

    assert_eq!(
        f.caller
            .try_on_transfer_acknowledgement(&impostor, &acknowledged),
        Err(Ok(Error::UntrustedTransferApp.into()))
    );
    assert_eq!(
        f.caller.try_on_transfer_timeout(&impostor, &timed_out),
        Err(Ok(Error::UntrustedTransferApp.into()))
    );
}
