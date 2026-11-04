use ipsa_poll::{
    calldata, voter_salt, Error, GmpAcknowledged, IpsaPoll, IpsaPollClient, Outcome,
    REMOTE_VOTE_TIMEOUT_SECONDS,
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

const REMOTE_POLL: &str = "0xF2C2474d14c2A68Ba49aBE709B12e1D3db12772F";

const VOTE_TWO_ONE: &str = "cff9293a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001";

const NOW: u64 = 1_000;

struct Fixture {
    env: Env,
    router: IbcRouterClient<'static>,
    gmp: IbcGmpAppClient<'static>,
    poll: IpsaPollClient<'static>,
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

    let poll_id = env.register(
        IpsaPoll,
        (
            owner,
            gmp_id.clone(),
            client_id.clone(),
            String::from_str(&env, REMOTE_POLL),
        ),
    );

    Fixture {
        router,
        gmp: IbcGmpAppClient::new(&env, &gmp_id),
        poll: IpsaPollClient::new(&env, &poll_id),
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

fn packet(f: &Fixture, sequence: u64, voter: &Address) -> Packet {
    let encoding = String::from_str(&f.env, SOLIDITY_ABI);
    let packet_data = GmpPacketData {
        sender: f.poll.address.to_string(),
        receiver: String::from_str(&f.env, REMOTE_POLL),
        salt: voter_salt(&f.env, voter),
        payload: hex(&f.env, VOTE_TWO_ONE),
        memo: String::from_str(&f.env, ""),
    };

    Packet {
        sequence,
        source_client: f.client_id.clone(),
        dest_client: f.counterparty_id.clone(),
        timeout_timestamp: NOW + REMOTE_VOTE_TIMEOUT_SECONDS,
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
fn the_calldata_is_the_solidity_vote_call() {
    let env = Env::default();

    assert_eq!(calldata(&env, 2, 1), hex(&env, VOTE_TWO_ONE));
}

#[test]
fn the_salt_is_the_voters_address() {
    let env = Env::default();
    let voter = Address::generate(&env);
    let salt = voter_salt(&env, &voter);
    let text = voter.to_string();
    let mut expected = [0u8; 56];

    text.copy_into_slice(&mut expected);

    assert_eq!(salt, Bytes::from_slice(&env, &expected));
}

#[test]
fn a_remote_vote_commits_the_packet_and_is_pending() {
    let f = setup();
    let voter = Address::generate(&f.env);

    let sequence = f.poll.vote_remote(&voter, &2, &1);
    let vote = f.poll.remote_vote(&sequence).unwrap();

    assert_eq!(vote.voter, voter);
    assert_eq!(vote.proposal, 2);
    assert_eq!(vote.option, 1);
    assert_eq!(vote.outcome, Outcome::Pending);
    assert!(f
        .router
        .packet_commitment(&f.client_id, &sequence)
        .is_some());

    acknowledge(
        &f,
        &packet(&f, sequence, &voter),
        abi_acknowledgement(&f.env, &abi_word(&f.env, 7)),
    );
}

#[test]
fn a_counted_vote_records_the_remote_tally() {
    let f = setup();
    let voter = Address::generate(&f.env);
    let sequence = f.poll.vote_remote(&voter, &2, &1);

    acknowledge(
        &f,
        &packet(&f, sequence, &voter),
        abi_acknowledgement(&f.env, &abi_word(&f.env, 7)),
    );

    assert_eq!(
        f.poll.remote_vote(&sequence).unwrap().outcome,
        Outcome::Counted(7)
    );
}

#[test]
fn a_refused_vote_is_recorded_as_rejected() {
    let f = setup();
    let voter = Address::generate(&f.env);
    let sequence = f.poll.vote_remote(&voter, &2, &1);

    acknowledge(
        &f,
        &packet(&f, sequence, &voter),
        error_acknowledgement(&f.env),
    );

    assert_eq!(
        f.poll.remote_vote(&sequence).unwrap().outcome,
        Outcome::Rejected
    );
}

#[test]
fn a_timed_out_vote_is_recorded_as_timed_out() {
    let f = setup();
    let voter = Address::generate(&f.env);
    let sequence = f.poll.vote_remote(&voter, &2, &1);
    let sent = packet(&f, sequence, &voter);
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
        f.poll.remote_vote(&sequence).unwrap().outcome,
        Outcome::TimedOut
    );
}

#[test]
fn a_remote_vote_needs_the_voters_authorisation() {
    let f = setup();

    f.env.set_auths(&[]);

    assert!(f
        .poll
        .try_vote_remote(&Address::generate(&f.env), &0, &0)
        .is_err());
    assert!(f.router.packet_commitment(&f.client_id, &1).is_none());
}

#[test]
fn a_callback_from_another_contract_is_refused() {
    let f = setup();
    let result = GmpAcknowledged {
        source_client: f.client_id.clone(),
        dest_client: f.counterparty_id.clone(),
        sequence: 1,
        success: true,
        result: Bytes::new(&f.env),
        acknowledgement: Bytes::new(&f.env),
    };

    assert_eq!(
        f.poll
            .try_on_gmp_acknowledgement(&Address::generate(&f.env), &result),
        Err(Ok(Error::UntrustedGmpApp.into()))
    );
}
