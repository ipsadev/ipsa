# ipsa-dev-kit

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

The crate is `no_std`. The Soroban modules depend only on `soroban-sdk`, and `core` only on `sha2`, `ics23` and `prost`. A Soroban contract turns `core` off so its wasm carries none of it:

```toml
ipsa = { version = "0.1", default-features = false, features = ["gmp"] }
```

A service that only builds or checks proofs does the opposite:

```toml
ipsa = { version = "0.1", default-features = false, features = ["core"] }
```

`ipsa::core` was published on its own as `ipsa-ibc-core` up to 0.1.2, from the `ipsa-core` repository. It lives here now.

The ipsa contracts in `ipsa-contracts` are built on this crate, so the types here are the ones on chain. A contract that calls the router, the transfer app or the GMP app, or is called back by them, should use them rather than copy them: a copy that misses a field stops decoding the callback, and Soroban reports nothing.

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

`just ci` runs formatting, fetches the contracts the examples import, then clippy, the tests, the Foundry tests of the issuer token and the Ethereum poll, and `cargo audit`, as CI does. It needs Foundry for those. `just package` builds what crates.io would receive without publishing it.

## Releasing

Push a tag `vX.Y.Z` matching the crate version. `.github/workflows/release.yml` runs `just ci` and `just package`, publishes `ipsa` to crates.io with the `CARGO_REGISTRY_TOKEN` secret, and writes the GitHub release. Running it by hand with `dry_run` rehearses every step without publishing. A version already on crates.io is only re-announced, and only if it was published from the same commit.

## Licence

Apache-2.0. See `LICENSE` and `NOTICE`.
