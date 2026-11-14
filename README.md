# ipsa

What a developer needs to build on ipsa, the IBC v2 implementation for Stellar.

## The `ipsa` crate

One crate, one module per area, each behind a feature of the same name. All are on by default.

| Module | Feature | For |
|---|---|---|
| `ipsa::core` | `core` | commitment paths, the fixed-depth SMT and its ICS-23 proofs: the bytes both sides of a corridor must compute the same way |
| `ipsa::types` | `types` | the packet, payload and callback types the Soroban router and its applications exchange |
| `ipsa::app` | `app` | a client for the Soroban router, and the `IbcApplication` interface a custom application implements |
| `ipsa::gmp` | `gmp` | the ICS-27 GMP types, a client for the GMP app, the `GmpCaller` callbacks and `remote_account` |
| `ipsa::ics20` | `ics20` | the ICS-20 types, a client for the transfer app, and the `TransferSender` and `TransferReceiver` callbacks |
| `ipsa::gateway` | `gateway` | gRPC clients and servers for the proof API (`ipsa::gateway::proofapi`) and the Stellar gateway (`ipsa::gateway::stellar`), generated from `proto/` |

The crate is `no_std` unless `gateway` is on, and `gateway` is off by default. The Soroban modules depend only on `soroban-sdk`, and `core` only on `sha2`, `ics23` and `prost`. A Soroban contract turns `core` off so its wasm carries none of it:

```toml
ipsa = { version = "0.2", default-features = false, features = ["gmp"] }
```

A service that only builds or checks proofs does the opposite:

```toml
ipsa = { version = "0.2", default-features = false, features = ["core"] }
```

`ipsa::core` was published on its own as `ipsa-ibc-core` up to 0.1.2, from the `ipsa-core` repository. It lives here now.

The ipsa contracts in `ipsa-contracts` are built on this crate, so the types here are the ones on chain. A contract that calls the router, the transfer app or the GMP app, or is called back by them, should use them rather than copy them: a copy that misses a field stops decoding the callback, and Soroban reports nothing.

## Talking to the proof API and the gateway

`proto/` is the one copy of the proof API and Stellar gateway protocols. `ipsa-gateway` serves them, and the relayer, the `ipsa` CLI and anyone relaying for themselves call them. `gateway` generates the clients and servers at build time with `protox`, so no `protoc` is needed:

```toml
ipsa = { version = "0.2", default-features = false, features = ["gateway"] }
```

```rust
use ipsa::gateway::proofapi::{proof_api_service_client::ProofApiServiceClient, RelayByTxRequest};

let mut proof_api = ProofApiServiceClient::connect("http://localhost:3100").await?;

let relay = proof_api
    .relay_by_tx(RelayByTxRequest {
        src_chain: "stellar-testnet".into(),
        dst_chain: "provider".into(),
        source_tx_ids: vec![hex::decode(transaction_hash)?],
        ..Default::default()
    })
    .await?
    .into_inner();
```

`RelayByTx` takes the source transactions to relay, and the destination transactions to time out, between two chains. It returns what the caller signs and submits, and the contract it goes to. For Stellar, `tx` is a length-prefixed XDR array of `HostFunction`, submitted one transaction at a time and in order. For the Cosmos Hub it is the Cosmos messages, and for an EVM chain the router calldata. `UpdateClient` and `CreateClient` return the transaction that updates or creates a client on the destination chain, and `Info` the chain ids, IBC version and router of the pair.

The generated messages use prost 0.14 while `core` stays on prost 0.13 for `ics23`, so encode and decode them with a prost 0.14 `Message`.

## Calling another chain from a Soroban contract

```rust
use ipsa::gmp::{GmpAcknowledged, GmpCaller, GmpTimedOut, IbcGmpClient};

let sequence = IbcGmpClient::new(&env, &gmp).send_call(
    &env.current_contract_address(),
    &client_id,
    &receiver,
    &salt,
    &payload,
    &timeout_timestamp,
    &memo,
    &encoding,
);
```

The GMP app then calls the contract back through `GmpCaller`. The contract should check that the caller is the GMP app it trusts and call `gmp.require_auth()`.

## Accepting calls from another chain

A call from another chain runs from an account derived from the client, the remote sender and the salt. Compute it once when the contract is set up, store it, and require its authorisation in each function the remote sender may call:

```rust
let remote = ipsa::gmp::remote_account(&env, &gmp, &client_id, &sender, &salt);

remote.require_auth();
```

The contract cannot ask the GMP app during the call: Soroban does not let a contract be re-entered while it runs.

## Examples

Small apps a developer could build on ipsa, around one story: governance on one chain acting on another, and an issuer controlling their asset on another chain. Each is tested against the real router and apps, imported as wasm from an `ipsa-contracts` release, the way a project outside ipsa would test.

- `examples/governance/dispatcher`: governance on Stellar acting on other chains. It sends approved actions to contracts on other chains through GMP and records each outcome, from pending to executed, rejected or timed out. Its owner is the governance contract, so only an approved proposal dispatches; the ipsa CLI deploys it with the admin as owner to test the GMP corridors. The council that votes on proposals and the executor on the other chain are still to come.
- `examples/governance/poll` and `examples/governance/ethereum-poll`: a poll on each chain that accepts votes from its own accounts and, through GMP, from accounts on the other chain. A remote voter votes as the account the GMP app derives for it, so it can vote only as itself, and local and remote votes share one tally.
- `examples/remote-treasury`: the same story the other way round, a treasury on Stellar governed by a DAO on another chain. Only the DAO's account, derived with `ipsa::gmp::remote_account`, can change it.
- `examples/payouts`: how a treasury pays people on other chains through ICS-20, recording each payout as paid, refunded or timed out.
- `examples/issuer`: how a Stellar asset issuer takes their asset to Ethereum without a bridge. The asset is escrowed on Stellar and arrives as the issuer's own ERC-20, which the issuer freezes and pauses from Stellar through GMP.

The protocol contracts themselves (router, transfer and GMP apps, light clients) live in `ipsa-contracts`. What lives here is what a developer copies and deploys for their own application.

```sh
just contracts                                  # the CONTRACTS_VERSION release into contracts/
CONTRACTS_WASM_DIR=../ipsa-contracts/target/wasm32v1-none/contract just contracts
just build-examples                             # the examples' wasm, for deploying
```

## Development

```sh
just ci
just package
```

`just ci` runs formatting, checks the contract wasm the examples import against `contracts/SHA256SUMS`, then clippy, the tests, the Foundry tests of the issuer token and the Ethereum poll, and `cargo audit`, as CI does. It needs Foundry for those. `just package` builds what crates.io would receive without publishing it, and `just package ipsa-chains` does the same for `ipsa-chains`.

`contracts/` holds the four `ipsa-contracts` wasm files the examples' tests import (the router, the transfer app, the GMP app and the GMP account) from release `v0.2.0`, with the release's checksums for them, so the tests run from a fresh clone. `just update-contracts <tag>` replaces them with another release's, checked against its `SHA256SUMS`, and `CONTRACTS_WASM_DIR` points `just contracts` at a local build instead. The files are not part of the published crate.

`cargo audit` ignores RUSTSEC-2026-0387, timing variability in `crypto-bigint` 0.5. It reaches the workspace only through `soroban-sdk` 26 (`soroban-env-host`, `k256`, `elliptic-curve`), and the fix is in `crypto-bigint` 0.7, which the Soroban SDK has not adopted yet. Drop the exception when it does.

It also ignores two advisories that reach only `ipsa-chains`: RUSTSEC-2026-0009, a stack exhaustion in `time` 0.3.37, which `ibc` 0.57 pins below the fixed 0.3.47, so `ipsa-chains` cannot move until `ibc` does, and RUSTSEC-2026-0258, unbounded empty DATA frames in `h2` 0.3, which comes with `soroban-client` 0.7's HTTP client and only talks to the Soroban RPC its caller configures. Drop each once `ibc` or `soroban-client` upgrades.

## Releasing

Each crate has its own tags: `vX.Y.Z` releases `ipsa` and `ipsa-chains-vX.Y.Z` releases `ipsa-chains`, each matching its crate's version. `.github/workflows/release.yml` runs `just ci` and `just package` for that crate, publishes it to crates.io with the `CARGO_REGISTRY_TOKEN` secret, and writes the GitHub release. Running it by hand with `dry_run`, and the crate to rehearse, runs every step without publishing. A version already on crates.io is only re-announced, and only if it was published from the same commit.

## Licence

Copyright 2026 Amanda Gonsalves.

Licensed under the Apache License, Version 2.0. See `LICENSE` and `NOTICE`. The contract wasm in `contracts/` is built from `ipsa-contracts`, which is under the same licence.
