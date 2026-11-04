# `GovernancePoll`: the Ethereum side of the poll

The twin of `examples/governance/poll` on Ethereum, a Foundry project. The same proposals and rules: the owner proposes, every address has one vote per proposal that it can move, and `vote` returns the option's new tally.

## Voting from Stellar

A Stellar account votes by calling `vote_remote(voter, proposal, option)` on the Stellar poll, which sends `vote(uint32 proposalId, uint32 option)` here through GMP with itself as the sender and the voter's address as the salt.

`ICS27GMP` runs the call from the account it derives for that sender and salt, so each Stellar voter is counted under their own account. `ICS27GMP.getOrComputeAccountAddress({ clientId: <Stellar client on Ethereum>, sender: <Stellar poll address>, salt: <voter's address as bytes> })` returns it before the first vote. The acknowledgement carries the new tally back to the Stellar poll.

## Deploying on testnet

```sh
forge soldeer install

POLL_OWNER=<owner address> \
forge script script/DeployGovernancePoll.s.sol --rpc-url $EVM_RPC_URL --private-key $EVM_PRIVATE_KEY --broadcast

cast send <poll> "propose(string,string[],uint64)" "Raise the treasury limit" '["Yes","No","Abstain"]' <unix seconds> \
  --rpc-url $EVM_RPC_URL --private-key $EVM_PRIVATE_KEY
```

## Tests

`forge test` covers proposals, one vote per voter, moved votes, closing, unknown proposals and options, and malformed input.
