use ipsa_issuer_console::{
    Action, Error, GmpAcknowledged, IpsaIssuerConsole, IpsaIssuerConsoleClient, Outcome,
    ACTION_TIMEOUT_SECONDS,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Bytes, Env, String,
};

mod router {
    soroban_sdk::contractimport!(file = "../../../contracts/stellar_ibc_router.wasm");
}

mod gmp {
    soroban_sdk::contractimport!(file = "../../../contracts/stellar_ibc_gmp.wasm");
}

use gmp::{
    Client as IbcGmpAppClient, GmpPacketData, OnTimeoutPacketCallback, Payload as GmpPayload,
};
use router::{Client as IbcRouterClient, Packet, Payload};

const SOLIDITY_ABI: &str = "application/x-solidity-abi";

const TOKEN: &str = "0x00000000000000000000000000000000000000aa";

const HOLDER: &str = "0x6943A601f20E774387D5D95912F1E8eB2745AB86";

const FREEZE_HOLDER: &str =
    "8d1fdf2f0000000000000000000000006943a601f20e774387d5d95912f1e8eb2745ab86";

const UNFREEZE_HOLDER: &str =
    "45c8b1a60000000000000000000000006943a601f20e774387d5d95912f1e8eb2745ab86";

const PAUSE: &str = "8456cb59";

const UNPAUSE: &str = "3f4ba83a";

const NOW: u64 = 1_000;

struct Fixture {
    env: Env,
    router: IbcRouterClient<'static>,
    gmp: IbcGmpAppClient<'static>,
    console: IpsaIssuerConsoleClient<'static>,
    client_id: String,
    counterparty_id: String,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|ledger| ledger.timestamp = NOW);

    let owner = Address::generate(&env);
    let router_id = env.register(router::WASM, (owner.clone(),));
    let router = IbcRouterClient::new(&env, &router_id);
    let light_client = env.register(ipsa_test_light_client::TestLightClient, ());
    router.register_client_type(&String::from_str(&env, "test"), &light_client);

    let client_id = router.create_client(
        &String::from_str(&env, "test"),
        &Bytes::from_slice(&env, b"cs"),
        &Bytes::from_slice(&env, b"cons"),
        &1,
    );
    let counterparty_id = String::from_str(&env, "stellar-testnet-2");
    router.register_counterparty(
        &client_id,
        &counterparty_id,
        &vec![&env, Bytes::from_slice(&env, b"ibc")],
    );

    let gmp_id = env.register(gmp::WASM, (owner.clone(), router_id.clone()));
    router.register_port(&String::from_str(&env, "gmpport"), &gmp_id);

    let console_id = env.register(
        IpsaIssuerConsole,
        (
            owner,
            gmp_id.clone(),
            client_id.clone(),
            String::from_str(&env, TOKEN),
        ),
    );

    Fixture {
        router,
        gmp: IbcGmpAppClient::new(&env, &gmp_id),
        console: IpsaIssuerConsoleClient::new(&env, &console_id),
        client_id,
        counterparty_id,
        env,
    }
}

fn hex(env: &Env, text: &str) -> Bytes {
    let bytes: std::vec::Vec<u8> = (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect();

    Bytes::from_slice(env, &bytes)
}

fn packet(f: &Fixture, sequence: u64, calldata: &str) -> Packet {
    let encoding = String::from_str(&f.env, SOLIDITY_ABI);
    let packet_data = GmpPacketData {
        sender: f.console.address.to_string(),
        receiver: String::from_str(&f.env, TOKEN),
        salt: Bytes::new(&f.env),
        payload: hex(&f.env, calldata),
        memo: String::from_str(&f.env, ""),
    };

    Packet {
        sequence,
        source_client: f.client_id.clone(),
        dest_client: f.counterparty_id.clone(),
        timeout_timestamp: NOW + ACTION_TIMEOUT_SECONDS,
        payloads: vec![
            &f.env,
            Payload {
                source_port: String::from_str(&f.env, "gmpport"),
                dest_port: String::from_str(&f.env, "gmpport"),
                version: String::from_str(&f.env, "ics27-2"),
                encoding: encoding.clone(),
                value: f.gmp.encode_packet(&encoding, &packet_data),
            },
        ],
    }
}

fn acknowledge(f: &Fixture, packet: &Packet, acknowledgement: Bytes) {
    f.router.acknowledge_packet(
        packet,
        &vec![&f.env, acknowledgement],
        &Bytes::from_slice(&f.env, b"proof"),
        &10,
    );
}

fn abi_word(env: &Env, value: u32) -> Bytes {
    let mut word = [0u8; 32];
    word[28..].copy_from_slice(&value.to_be_bytes());

    Bytes::from_array(env, &word)
}

fn abi_acknowledgement(env: &Env, result: &Bytes) -> Bytes {
    let mut out = abi_word(env, 32);
    let padding = (32 - result.len() % 32) % 32;

    out.append(&abi_word(env, 32));
    out.append(&abi_word(env, result.len()));
    out.append(result);
    out.append(&Bytes::from_slice(env, &[0u8; 32][..padding as usize]));

    out
}

fn error_acknowledgement(env: &Env) -> Bytes {
    let hash = env
        .crypto()
        .sha256(&Bytes::from_slice(env, b"UNIVERSAL_ERROR_ACKNOWLEDGEMENT"))
        .to_array();

    Bytes::from_array(env, &hash)
}

#[test]
fn freezing_a_holder_sends_the_solidity_call_and_is_pending() {
    let f = setup();
    let holder = String::from_str(&f.env, HOLDER);

    let sequence = f.console.freeze(&holder);
    let record = f.console.record(&sequence).unwrap();

    assert_eq!(record.action, Action::Freeze(holder));
    assert_eq!(record.outcome, Outcome::Pending);
    assert!(f
        .router
        .packet_commitment(&f.client_id, &sequence)
        .is_some());

    acknowledge(
        &f,
        &packet(&f, sequence, FREEZE_HOLDER),
        abi_acknowledgement(&f.env, &Bytes::new(&f.env)),
    );
}

#[test]
fn an_executed_action_records_the_result() {
    let f = setup();
    let sequence = f.console.freeze(&String::from_str(&f.env, HOLDER));
    let result = Bytes::from_slice(&f.env, b"done");

    acknowledge(
        &f,
        &packet(&f, sequence, FREEZE_HOLDER),
        abi_acknowledgement(&f.env, &result),
    );

    assert_eq!(
        f.console.record(&sequence).unwrap().outcome,
        Outcome::Executed(result)
    );
}

#[test]
fn a_refused_action_is_recorded_as_rejected() {
    let f = setup();
    let sequence = f.console.unfreeze(&String::from_str(&f.env, HOLDER));

    acknowledge(
        &f,
        &packet(&f, sequence, UNFREEZE_HOLDER),
        error_acknowledgement(&f.env),
    );

    assert_eq!(
        f.console.record(&sequence).unwrap().outcome,
        Outcome::Rejected
    );
}

#[test]
fn pausing_and_unpausing_send_their_selectors() {
    let f = setup();

    let pause = f.console.pause();
    let unpause = f.console.unpause();

    acknowledge(
        &f,
        &packet(&f, pause, PAUSE),
        abi_acknowledgement(&f.env, &Bytes::new(&f.env)),
    );
    acknowledge(
        &f,
        &packet(&f, unpause, UNPAUSE),
        abi_acknowledgement(&f.env, &Bytes::new(&f.env)),
    );

    assert!(matches!(
        f.console.record(&pause).unwrap().outcome,
        Outcome::Executed(_)
    ));
    assert!(matches!(
        f.console.record(&unpause).unwrap().outcome,
        Outcome::Executed(_)
    ));
}

#[test]
fn a_timed_out_action_is_recorded_as_timed_out() {
    let f = setup();
    let sequence = f.console.pause();
    let sent = packet(&f, sequence, PAUSE);
    let payload = sent.payloads.get(0).unwrap();

    f.gmp.on_timeout_packet(&OnTimeoutPacketCallback {
        source_client: sent.source_client.clone(),
        dest_client: sent.dest_client.clone(),
        sequence,
        payload: GmpPayload {
            source_port: payload.source_port,
            dest_port: payload.dest_port,
            version: payload.version,
            encoding: payload.encoding,
            value: payload.value,
        },
    });

    assert_eq!(
        f.console.record(&sequence).unwrap().outcome,
        Outcome::TimedOut
    );
}

#[test]
fn a_malformed_address_is_refused_before_anything_is_sent() {
    let f = setup();

    let refused = f.console.try_freeze(&String::from_str(&f.env, "0x1234"));

    assert_eq!(refused, Err(Ok(Error::InvalidEvmAddress.into())));
    assert!(f.router.packet_commitment(&f.client_id, &1).is_none());
}

#[test]
fn a_callback_from_another_contract_is_refused() {
    let f = setup();
    let impostor = Address::generate(&f.env);
    let result = GmpAcknowledged {
        source_client: f.client_id.clone(),
        dest_client: f.counterparty_id.clone(),
        sequence: 1,
        success: true,
        result: Bytes::new(&f.env),
        acknowledgement: Bytes::new(&f.env),
    };

    let refused = f.console.try_on_gmp_acknowledgement(&impostor, &result);

    assert_eq!(refused, Err(Ok(Error::UntrustedGmpApp.into())));
}
