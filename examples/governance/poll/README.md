# `ipsa-poll`: a vote anyone can cast from another chain

A governance poll on Stellar that accepts votes from Stellar accounts and, through ICS-27 GMP, from accounts on Ethereum, and sends its own voters' votes to its twin on Ethereum, `examples/governance/ethereum-poll`. It is built on the `ipsa` crate's `gmp` module: `IbcGmpClient` to send, `GmpCaller` to hear the outcome, and `remote_account` to name a remote voter's account.

## How it works

- The owner opens a proposal with `propose(title, options, closes_at)`: a title of up to 140 bytes, two to eight options of up to 40 bytes, and a closing time in seconds.
- `vote(voter, proposal, option)` requires the voter's authorisation and returns the option's new tally. A voter has one vote per proposal; voting again for another option moves it, and voting again for the same option changes nothing.
- A Stellar account votes directly. An account on another chain votes through its GMP account, the contract the GMP app derives from `(client, sender, salt)` and deploys on first use. The GMP call's arguments name that derived account as the voter, and the GMP app authorises the call as it, so a remote sender can vote only as its own account. `account_address(client_id, sender, salt)` on the GMP app returns the address before the first vote.
- Local and remote votes count in the same tally. Each vote emits `voted` with the proposal, the voter, the option and the new tally.
- `proposal(id)`, `proposals()` and `vote_of(proposal, voter)` read the state. Proposals and votes are kept for 90 days from their last change.

## Voting on Ethereum from Stellar

- `vote_remote(voter, proposal, option)` requires the voter's authorisation and sends `vote(uint32,uint32)` to the Ethereum poll through the GMP app, with this contract as the sender and the voter's address as the salt. It returns the packet's sequence.
- On Ethereum, `ICS27GMP` runs the call from the account it derives for `(Stellar client on Ethereum, this contract, voter's address)`, so each Stellar voter has their own account there and one vote. The Ethereum poll trusts this contract to have checked the voter's authorisation, which `vote_remote` does before sending.
- The poll implements `GmpCaller`, so the GMP app reports back. `remote_vote(sequence)` reads the vote and its outcome: `Pending`, then `Counted(tally)` with the option's new tally on Ethereum, `Rejected` if the Ethereum poll refused it (closed, unknown proposal or option), or `TimedOut` after six hours undelivered. Callbacks from any contract other than the GMP app are refused.
- The constructor takes the owner, the GMP app, the Ethereum client on Stellar and the Ethereum poll's address; `set_remote_poll` changes the address.

## Voting from Ethereum

The voter calls `ICS27GMP.sendCall` on Ethereum with:

- `sourceClient`: the Stellar client on Ethereum.
- `receiver`: this contract's address.
- `salt`: empty.
- `payload`: the XDR of the `GmpCall { target: <this contract>, function: "vote", args: [<voter's derived account>, <proposal>, <option>] }` map.

The sender the GMP app sees is the voter's checksummed Ethereum address, so the derived account is `voter_account(<checksummed address>)`, which computes it with `ipsa::gmp::remote_account` and matches the GMP app's own `account_address`.

## Deploying on testnet

```sh
just build-examples

stellar contract deploy --source admin --network testnet \
  --wasm target/wasm32v1-none/contract/ipsa_poll.wasm \
  -- --owner <owner account> --gmp $GMP_CONTRACT_ADDRESS --client_id $STELLAR_EVM_CLIENT_ID --poll <Ethereum poll address>

stellar contract invoke --source admin --network testnet --id <poll> -- \
  propose --title "Raise the treasury limit" --options '["Yes","No","Abstain"]' --closes_at <unix seconds>
```

## Tests

`tests/votes.rs` covers proposals, one vote per voter, moved votes, closing and malformed input. `tests/votes_from_ethereum.rs` runs votes through the router and the GMP app as packets from Ethereum: a remote vote through the derived account, a remote sender refused when it names another account, `voter_account` matching the GMP app, and local and remote votes sharing one tally. `tests/votes_to_ethereum.rs` sends votes to Ethereum: the calldata and salt, the committed packet, and each outcome from the acknowledgement or timeout, including a refused impostor callback.
