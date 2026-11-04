use ipsa_poll::{Error, IpsaPoll, IpsaPollClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, String, Vec,
};

const NOW: u64 = 1_000;

const REMOTE_POLL: &str = "0xF2C2474d14c2A68Ba49aBE709B12e1D3db12772F";

const CLOSES_AT: u64 = 5_000;

fn setup() -> (Env, IpsaPollClient<'static>, Address) {
    let env = Env::default();

    env.mock_all_auths();
    env.ledger().with_mut(|ledger| ledger.timestamp = NOW);

    let owner = Address::generate(&env);
    let id = env.register(
        IpsaPoll,
        (
            owner.clone(),
            Address::generate(&env),
            String::from_str(&env, "sepolia-0"),
            String::from_str(&env, REMOTE_POLL),
        ),
    );
    let poll = IpsaPollClient::new(&env, &id);

    (env, poll, owner)
}

fn options(env: &Env) -> Vec<String> {
    vec![
        env,
        String::from_str(env, "Yes"),
        String::from_str(env, "No"),
        String::from_str(env, "Abstain"),
    ]
}

fn propose(env: &Env, poll: &IpsaPollClient) -> u32 {
    poll.propose(
        &String::from_str(env, "Raise the treasury limit"),
        &options(env),
        &CLOSES_AT,
    )
}

#[test]
fn a_proposal_starts_with_empty_tallies() {
    let (env, poll, _) = setup();
    let id = propose(&env, &poll);
    let proposal = poll.proposal(&id);

    assert_eq!(id, 0);
    assert_eq!(proposal.tallies, vec![&env, 0, 0, 0]);
    assert_eq!(proposal.voters, 0);
    assert_eq!(poll.proposals().len(), 1);
}

#[test]
fn only_the_owner_proposes() {
    let (env, poll, _) = setup();

    env.set_auths(&[]);

    assert!(poll
        .try_propose(
            &String::from_str(&env, "Raise the treasury limit"),
            &options(&env),
            &CLOSES_AT,
        )
        .is_err());
}

#[test]
fn a_vote_counts_once_per_voter() {
    let (env, poll, _) = setup();
    let id = propose(&env, &poll);
    let voter = Address::generate(&env);

    assert_eq!(poll.vote(&voter, &id, &0), 1);
    assert_eq!(poll.vote(&voter, &id, &0), 1);
    assert_eq!(poll.vote(&Address::generate(&env), &id, &0), 2);

    let proposal = poll.proposal(&id);

    assert_eq!(proposal.tallies, vec![&env, 2, 0, 0]);
    assert_eq!(proposal.voters, 2);
    assert_eq!(poll.vote_of(&id, &voter), Some(0));
}

#[test]
fn a_changed_vote_moves_between_options() {
    let (env, poll, _) = setup();
    let id = propose(&env, &poll);
    let voter = Address::generate(&env);

    poll.vote(&voter, &id, &0);

    assert_eq!(poll.vote(&voter, &id, &1), 1);

    let proposal = poll.proposal(&id);

    assert_eq!(proposal.tallies, vec![&env, 0, 1, 0]);
    assert_eq!(proposal.voters, 1);
}

#[test]
fn a_vote_needs_the_voters_authorisation() {
    let (env, poll, _) = setup();
    let id = propose(&env, &poll);

    env.set_auths(&[]);

    assert!(poll.try_vote(&Address::generate(&env), &id, &0).is_err());
}

#[test]
fn a_vote_after_closing_is_refused() {
    let (env, poll, _) = setup();
    let id = propose(&env, &poll);

    env.ledger().with_mut(|ledger| ledger.timestamp = CLOSES_AT);

    assert_eq!(
        poll.try_vote(&Address::generate(&env), &id, &0),
        Err(Ok(Error::Closed.into()))
    );
}

#[test]
fn an_unknown_proposal_or_option_is_refused() {
    let (env, poll, _) = setup();
    let id = propose(&env, &poll);
    let voter = Address::generate(&env);

    assert_eq!(
        poll.try_vote(&voter, &(id + 1), &0),
        Err(Ok(Error::UnknownProposal.into()))
    );
    assert_eq!(
        poll.try_vote(&voter, &id, &3),
        Err(Ok(Error::UnknownOption.into()))
    );
}

#[test]
fn a_malformed_remote_poll_address_is_refused() {
    let (env, poll, _) = setup();

    assert_eq!(
        poll.try_set_remote_poll(&String::from_str(&env, "0x1234")),
        Err(Ok(Error::InvalidEvmAddress.into()))
    );
    assert_eq!(
        poll.try_set_remote_poll(&String::from_str(
            &env,
            "0xZZC2474d14c2A68Ba49aBE709B12e1D3db12772F"
        )),
        Err(Ok(Error::InvalidEvmAddress.into()))
    );
}

#[test]
fn a_malformed_proposal_is_refused() {
    let (env, poll, _) = setup();
    let title = String::from_str(&env, "Raise the treasury limit");
    let one_option = vec![&env, String::from_str(&env, "Yes")];

    assert_eq!(
        poll.try_propose(&String::from_str(&env, ""), &options(&env), &CLOSES_AT),
        Err(Ok(Error::InvalidTitle.into()))
    );
    assert_eq!(
        poll.try_propose(&title, &one_option, &CLOSES_AT),
        Err(Ok(Error::InvalidOptions.into()))
    );
    assert_eq!(
        poll.try_propose(&title, &options(&env), &NOW),
        Err(Ok(Error::InvalidClosingTime.into()))
    );
}
