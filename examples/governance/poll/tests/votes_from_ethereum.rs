mod router {
    soroban_sdk::contractimport!(file = "../../../contracts/stellar_ibc_router.wasm");
}

mod gmp {
    soroban_sdk::contractimport!(file = "../../../contracts/stellar_ibc_gmp.wasm");
}

mod gmp_account {
    soroban_sdk::contractimport!(file = "../../../contracts/ipsa_gmp_account.wasm");
}

use ipsa_poll::{IpsaPoll, IpsaPollClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};

const VOTER: &str = "0x06eCF411bE934d0eCb80a247508cA96c8030808E";

const COUNTERPARTY_CLIENT: &str = "stellar-testnet-2";

struct Fixture {
    env: Env,
    router: router::Client<'static>,
    gmp: gmp::Client<'static>,
    poll: IpsaPollClient<'static>,
    client_id: String,
    sequence: u64,
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

    let gmp_id = env.register(gmp::WASM, (owner.clone(), router_id));
    let gmp = gmp::Client::new(&env, &gmp_id);

    gmp.set_account_wasm(&env.deployer().upload_contract_wasm(gmp_account::WASM));
    router.register_port(&String::from_str(&env, "gmpport"), &gmp_id);

    let poll_id = env.register(
        IpsaPoll,
        (
            owner,
            gmp_id.clone(),
            client_id.clone(),
            String::from_str(&env, "0xF2C2474d14c2A68Ba49aBE709B12e1D3db12772F"),
        ),
    );
    let poll = IpsaPollClient::new(&env, &poll_id);

    poll.propose(
        &String::from_str(&env, "Raise the treasury limit"),
        &vec![
            &env,
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
        &5_000,
    );

    Fixture {
        env,
        router,
        gmp,
        poll,
        client_id,
        sequence: 0,
    }
}

fn derived_account(f: &Fixture, sender: &str) -> Address {
    f.gmp
        .account_address(
            &f.client_id,
            &String::from_str(&f.env, sender),
            &Bytes::new(&f.env),
        )
        .address
}

fn remote_vote(f: &mut Fixture, sender: &str, voter: &Address, option: u32) -> Option<BytesN<32>> {
    let args: Vec<Val> = vec![
        &f.env,
        voter.into_val(&f.env),
        0u32.into_val(&f.env),
        option.into_val(&f.env),
    ];
    let call = ipsa::gmp::GmpCall {
        target: f.poll.address.clone(),
        function: Symbol::new(&f.env, "vote"),
        args,
    };
    let data = gmp::GmpPacketData {
        sender: String::from_str(&f.env, sender),
        receiver: String::from_str(&f.env, ""),
        salt: Bytes::new(&f.env),
        payload: call.to_xdr(&f.env),
        memo: String::from_str(&f.env, ""),
    };
    let encoding = String::from_str(&f.env, ipsa::gmp::ENCODING_SOLIDITY_ABI);

    f.sequence += 1;

    f.router.recv_packet(
        &router::Packet {
            sequence: f.sequence,
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

    f.router.acknowledgement(&f.client_id, &f.sequence)
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
fn a_remote_sender_votes_through_its_derived_account() {
    let mut f = setup();
    let account = derived_account(&f, VOTER);

    f.env.set_auths(&[]);

    let acknowledgement = remote_vote(&mut f, VOTER, &account, 1);

    assert_ne!(
        acknowledgement,
        Some(error_acknowledgement_commitment(&f.env))
    );
    assert_eq!(f.poll.vote_of(&0, &account), Some(1));
    assert_eq!(f.poll.proposal(&0).tallies, vec![&f.env, 0, 1]);
}

#[test]
fn a_remote_sender_cannot_vote_as_another_account() {
    let mut f = setup();
    let someone_else = derived_account(&f, "0x0000000000000000000000000000000000000001");

    f.env.set_auths(&[]);

    let acknowledgement = remote_vote(&mut f, VOTER, &someone_else, 0);

    assert_eq!(
        acknowledgement,
        Some(error_acknowledgement_commitment(&f.env))
    );
    assert_eq!(f.poll.proposal(&0).tallies, vec![&f.env, 0, 0]);
}

#[test]
fn the_poll_derives_the_same_voter_account_as_the_gmp_app() {
    let f = setup();

    assert_eq!(
        f.poll.voter_account(&String::from_str(&f.env, VOTER)),
        derived_account(&f, VOTER)
    );
}

#[test]
fn local_and_remote_votes_share_one_tally() {
    let mut f = setup();
    let account = derived_account(&f, VOTER);

    f.poll.vote(&Address::generate(&f.env), &0, &0);

    f.env.set_auths(&[]);

    remote_vote(&mut f, VOTER, &account, 0);

    let proposal = f.poll.proposal(&0);

    assert_eq!(proposal.tallies, vec![&f.env, 2, 0]);
    assert_eq!(proposal.voters, 2);
}
