#![no_std]

pub use ipsa::gmp::{GmpAcknowledged, GmpTimedOut};
use ipsa::gmp::{GmpCaller, IbcGmpClient, ENCODING_SOLIDITY_ABI};
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    Bytes, Env, String, Vec,
};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_macros::only_owner;

pub const MAXIMUM_TITLE_LENGTH: u32 = 140;

pub const MAXIMUM_OPTION_LENGTH: u32 = 40;

pub const MINIMUM_OPTIONS: u32 = 2;

pub const MAXIMUM_OPTIONS: u32 = 8;

pub const REMOTE_VOTE_TIMEOUT_SECONDS: u64 = 6 * 60 * 60;

pub const REMOTE_VOTE_SIGNATURE: &str = "vote(uint32,uint32)";

const DAY_IN_LEDGERS: u32 = 17_280;

const KEEP_FOR: u32 = 90 * DAY_IN_LEDGERS;

const EXTEND_BELOW: u32 = 30 * DAY_IN_LEDGERS;

const EVM_ADDRESS_LENGTH: usize = 42;

const ABI_WORD_LENGTH: u32 = 32;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    UnknownProposal = 1,
    UnknownOption = 2,
    Closed = 3,
    InvalidTitle = 4,
    InvalidOptions = 5,
    InvalidClosingTime = 6,
    UntrustedGmpApp = 7,
    InvalidEvmAddress = 8,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u32,
    pub title: String,
    pub options: Vec<String>,
    pub tallies: Vec<u32>,
    pub voters: u32,
    pub closes_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Remote {
    pub gmp: Address,
    pub client_id: String,
    pub poll: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pending,
    Counted(u32),
    Rejected,
    TimedOut,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteVote {
    pub voter: Address,
    pub proposal: u32,
    pub option: u32,
    pub outcome: Outcome,
}

#[contractevent(topics = ["proposed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposed {
    #[topic]
    pub proposal: u32,
    pub title: String,
    pub closes_at: u64,
}

#[contractevent(topics = ["voted"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Voted {
    #[topic]
    pub proposal: u32,
    #[topic]
    pub voter: Address,
    pub option: u32,
    pub tally: u32,
}

#[contractevent(topics = ["remote_vote"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteVoteSent {
    #[topic]
    pub sequence: u64,
    #[topic]
    pub voter: Address,
    pub proposal: u32,
    pub option: u32,
}

#[contractevent(topics = ["remote_vote_settled"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteVoteSettled {
    #[topic]
    pub sequence: u64,
    pub outcome: Outcome,
}

#[contracttype]
enum DataKey {
    Remote,
    Count,
    Proposal(u32),
    Vote(u32, Address),
    RemoteVote(u64),
}

#[contract]
pub struct IpsaPoll;

#[contractimpl(contracttrait)]
impl Ownable for IpsaPoll {}

#[contractimpl]
impl IpsaPoll {
    pub fn __constructor(env: Env, owner: Address, gmp: Address, client_id: String, poll: String) {
        evm_address(&env, &poll);
        set_owner(&env, &owner);
        env.storage().instance().set(
            &DataKey::Remote,
            &Remote {
                gmp,
                client_id,
                poll,
            },
        );
    }

    #[only_owner]
    pub fn propose(env: Env, title: String, options: Vec<String>, closes_at: u64) -> u32 {
        if title.is_empty() || title.len() > MAXIMUM_TITLE_LENGTH {
            panic_with_error!(&env, Error::InvalidTitle);
        }

        if options.len() < MINIMUM_OPTIONS
            || options.len() > MAXIMUM_OPTIONS
            || options
                .iter()
                .any(|option| option.is_empty() || option.len() > MAXIMUM_OPTION_LENGTH)
        {
            panic_with_error!(&env, Error::InvalidOptions);
        }

        if closes_at <= env.ledger().timestamp() {
            panic_with_error!(&env, Error::InvalidClosingTime);
        }

        let id: u32 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let mut tallies = Vec::new(&env);

        for _ in 0..options.len() {
            tallies.push_back(0);
        }

        let proposal = Proposal {
            id,
            title: title.clone(),
            options,
            tallies,
            voters: 0,
            closes_at,
        };

        store_proposal(&env, &proposal);
        env.storage().instance().set(&DataKey::Count, &(id + 1));
        env.storage().instance().extend_ttl(EXTEND_BELOW, KEEP_FOR);

        Proposed {
            proposal: id,
            title,
            closes_at,
        }
        .publish(&env);

        id
    }

    #[only_owner]
    pub fn set_remote_poll(env: Env, poll: String) {
        evm_address(&env, &poll);

        let mut remote = remote(&env);

        remote.poll = poll;
        env.storage().instance().set(&DataKey::Remote, &remote);
    }

    pub fn vote(env: Env, voter: Address, proposal: u32, option: u32) -> u32 {
        voter.require_auth();

        let mut stored = load_proposal(&env, proposal);

        if env.ledger().timestamp() >= stored.closes_at {
            panic_with_error!(&env, Error::Closed);
        }

        if option >= stored.options.len() {
            panic_with_error!(&env, Error::UnknownOption);
        }

        let key = DataKey::Vote(proposal, voter.clone());
        let previous: Option<u32> = env.storage().persistent().get(&key);

        match previous {
            Some(earlier) if earlier == option => {
                return stored.tallies.get(option).unwrap();
            }
            Some(earlier) => {
                let count = stored.tallies.get(earlier).unwrap();

                stored.tallies.set(earlier, count - 1);
            }
            None => stored.voters += 1,
        }

        let tally = stored.tallies.get(option).unwrap() + 1;

        stored.tallies.set(option, tally);
        store_proposal(&env, &stored);
        env.storage().persistent().set(&key, &option);
        env.storage()
            .persistent()
            .extend_ttl(&key, EXTEND_BELOW, KEEP_FOR);

        Voted {
            proposal,
            voter,
            option,
            tally,
        }
        .publish(&env);

        tally
    }

    pub fn vote_remote(env: Env, voter: Address, proposal: u32, option: u32) -> u64 {
        voter.require_auth();

        let remote = remote(&env);

        let sequence = IbcGmpClient::new(&env, &remote.gmp).send_call(
            &env.current_contract_address(),
            &remote.client_id,
            &remote.poll,
            &voter_salt(&env, &voter),
            &calldata(&env, proposal, option),
            &(env.ledger().timestamp() + REMOTE_VOTE_TIMEOUT_SECONDS),
            &String::from_str(&env, ""),
            &String::from_str(&env, ENCODING_SOLIDITY_ABI),
        );

        let key = DataKey::RemoteVote(sequence);

        env.storage().persistent().set(
            &key,
            &RemoteVote {
                voter: voter.clone(),
                proposal,
                option,
                outcome: Outcome::Pending,
            },
        );
        env.storage()
            .persistent()
            .extend_ttl(&key, EXTEND_BELOW, KEEP_FOR);

        RemoteVoteSent {
            sequence,
            voter,
            proposal,
            option,
        }
        .publish(&env);

        sequence
    }

    pub fn proposal(env: Env, proposal: u32) -> Proposal {
        load_proposal(&env, proposal)
    }

    pub fn proposals(env: Env) -> Vec<Proposal> {
        let count: u32 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let mut proposals = Vec::new(&env);

        for id in 0..count {
            if let Some(proposal) = env.storage().persistent().get(&DataKey::Proposal(id)) {
                proposals.push_back(proposal);
            }
        }

        proposals
    }

    pub fn vote_of(env: Env, proposal: u32, voter: Address) -> Option<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::Vote(proposal, voter))
    }

    pub fn remote_vote(env: Env, sequence: u64) -> Option<RemoteVote> {
        env.storage()
            .persistent()
            .get(&DataKey::RemoteVote(sequence))
    }

    pub fn voter_account(env: Env, sender: String) -> Address {
        let remote = remote(&env);

        ipsa::gmp::remote_account(
            &env,
            &remote.gmp,
            &remote.client_id,
            &sender,
            &Bytes::new(&env),
        )
    }

    pub fn remote(env: Env) -> Remote {
        remote(&env)
    }
}

#[contractimpl]
impl GmpCaller for IpsaPoll {
    fn on_gmp_acknowledgement(env: Env, gmp: Address, result: GmpAcknowledged) {
        trusted(&env, &gmp);

        let outcome = if result.success {
            Outcome::Counted(tally_of(&result.result))
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

pub fn calldata(env: &Env, proposal: u32, option: u32) -> Bytes {
    let selector = env
        .crypto()
        .keccak256(&Bytes::from_slice(env, REMOTE_VOTE_SIGNATURE.as_bytes()))
        .to_array();
    let mut out = Bytes::from_slice(env, &selector[..4]);

    out.append(&abi_word(env, proposal));
    out.append(&abi_word(env, option));

    out
}

pub fn voter_salt(env: &Env, voter: &Address) -> Bytes {
    let text = voter.to_string();
    let mut buffer = [0u8; 56];
    let length = text.len() as usize;

    text.copy_into_slice(&mut buffer[..length]);

    Bytes::from_slice(env, &buffer[..length])
}

fn abi_word(env: &Env, value: u32) -> Bytes {
    let mut word = [0u8; 32];

    word[28..].copy_from_slice(&value.to_be_bytes());

    Bytes::from_array(env, &word)
}

fn tally_of(result: &Bytes) -> u32 {
    if result.len() != ABI_WORD_LENGTH {
        return 0;
    }

    let mut word = [0u8; 32];

    result.copy_into_slice(&mut word);

    u32::from_be_bytes([word[28], word[29], word[30], word[31]])
}

fn settle(env: &Env, sequence: u64, outcome: Outcome) {
    let key = DataKey::RemoteVote(sequence);

    if let Some(mut vote) = env.storage().persistent().get::<_, RemoteVote>(&key) {
        vote.outcome = outcome.clone();
        env.storage().persistent().set(&key, &vote);

        RemoteVoteSettled { sequence, outcome }.publish(env);
    }
}

fn trusted(env: &Env, gmp: &Address) {
    if *gmp != remote(env).gmp {
        panic_with_error!(env, Error::UntrustedGmpApp);
    }

    gmp.require_auth();
}

fn remote(env: &Env) -> Remote {
    env.storage().instance().get(&DataKey::Remote).unwrap()
}

fn load_proposal(env: &Env, proposal: u32) -> Proposal {
    env.storage()
        .persistent()
        .get(&DataKey::Proposal(proposal))
        .unwrap_or_else(|| panic_with_error!(env, Error::UnknownProposal))
}

fn store_proposal(env: &Env, proposal: &Proposal) {
    let key = DataKey::Proposal(proposal.id);

    env.storage().persistent().set(&key, proposal);
    env.storage()
        .persistent()
        .extend_ttl(&key, EXTEND_BELOW, KEEP_FOR);
}

fn evm_address(env: &Env, text: &String) {
    if text.len() as usize != EVM_ADDRESS_LENGTH {
        panic_with_error!(env, Error::InvalidEvmAddress);
    }

    let mut ascii = [0u8; EVM_ADDRESS_LENGTH];

    text.copy_into_slice(&mut ascii);

    if &ascii[..2] != b"0x" || !ascii[2..].iter().all(u8::is_ascii_hexdigit) {
        panic_with_error!(env, Error::InvalidEvmAddress);
    }
}
