# ipsa-chains

The Stellar and Cosmos encoding that ipsa's off-chain services share: Soroban values, IBC v2 messages, Tendermint client and consensus states, and the router's packet events. [ipsa-backend](https://github.com/ipsadev/ipsa-backend)'s api and provider and [ipsa-gateway](https://github.com/ipsadev/ipsa-gateway) depend on it. The primitives both chains must agree on (commitment paths, the sparse Merkle tree and its ICS-23 proofs) are not here: they are in the [`ipsa`](https://crates.io/crates/ipsa) crate's `core` module, in this same repository.

```toml
ipsa-chains = "0.1"
```

| Module | What it does |
|---|---|
| `conversion` | builds and reads Soroban `ScVal`s |
| `types` | the ledger, event and transaction shapes the api serves and the gateway reads |
| `msg`, `v2_msgs` | encodes IBC v2 messages and decodes packets and acknowledgements |
| `packet` | reads the router's packet events |
| `client_state`, `consensus_state` | the counterparty client's state, as Soroban stores it and as ibc-go reads it |
| `tendermint` | builds Tendermint headers, with their trusted validators, and the genesis of a Tendermint client for the Stellar router |
| `soroban` | converts Tendermint headers and states to and from Soroban XDR |

## Versions

This crate pins `prost` 0.13 and `ibc-proto` 0.51 because its modules pass protobuf types to one another and those types only implement the `prost::Message` of the version they were generated against. Move all of it together, or none.

## Revision numbers

`tendermint::revision_of` takes the revision from the chain id's trailing `-N`. ibc-go reads it the same way, and a client whose revision disagrees rejects every header it is later given.

## Trusted validators

A Tendermint header carries two validator sets: its own, and the one the Stellar light client already trusts, the set at the trusted height + 1. `tendermint::header_proto` takes both, and `soroban::tendermint_header_to_soroban_xdr` passes the trusted one to the light client as `trusted_validator_set_bytes`. When the chain's validator set has changed since the trusted height, the light client accepts the header only if validators from the trusted set holding more than its trust level of their power signed it; without that set it cannot follow the chain past the change. ipsa-backend's `/cosmos/header/{height}` reads the trusted set from the Cosmos RPC at the trusted height + 1, so that height must not be pruned on the node.

Light clients from before `trusted_validator_set_bytes` cannot decode a header that carries it, so a light client must be moved to new code before the gateway sends it. On staging, `07-tendermint-1` was migrated to such a light client before v1.0.15, the first release that sends the field.

## Releasing

Push a tag `ipsa-chains-vX.Y.Z` matching this crate's version; the repository's release workflow publishes it on its own, separately from `ipsa`.

## Licence

Copyright 2026 Amanda Gonsalves. Licensed under the Apache License, Version 2.0. See `LICENSE` and `NOTICE`.
