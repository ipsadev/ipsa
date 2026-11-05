# Crates for building on ipsa: IBC v2 types, the router client, GMP and ICS-20

lints := "-D warnings -A clippy::manual_is_multiple_of -A clippy::too_many_arguments -A clippy::result_large_err"

default:
    @just --list

contracts_repository := "ipsadev/ipsa-contracts"
contracts_version := env_var_or_default("CONTRACTS_VERSION", "v0.2.0")

# everything CI runs, in CI's order
ci: fmt-check contracts lint test solidity audit

contracts_files := "stellar_ibc_router.wasm stellar_ibc_transfer.wasm stellar_ibc_gmp.wasm ipsa_gmp_account.wasm"

# the ipsa contract wasm the examples' tests import: the committed copies, checked against SHA256SUMS, or a local build from CONTRACTS_WASM_DIR
contracts:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -n "${CONTRACTS_WASM_DIR:-}" ]; then
        for file in {{contracts_files}}; do cp "$CONTRACTS_WASM_DIR/$file" contracts/; done
        echo "copied the local build in $CONTRACTS_WASM_DIR"
        exit 0
    fi
    cd contracts && shasum -a 256 -c SHA256SUMS

# replace the committed contract wasm with an ipsa-contracts release's, checked against its SHA256SUMS
update-contracts version=contracts_version:
    #!/usr/bin/env bash
    set -euo pipefail
    scratch="$(mktemp -d)"
    trap 'rm -rf "$scratch"' EXIT
    gh release download "{{version}}" --repo "{{contracts_repository}}" --pattern '*.wasm' --pattern SHA256SUMS --dir "$scratch"
    (cd "$scratch" && shasum -a 256 -c SHA256SUMS --ignore-missing)
    : > contracts/SHA256SUMS
    for file in {{contracts_files}}; do
        cp "$scratch/$file" contracts/
        grep " $file$" "$scratch/SHA256SUMS" >> contracts/SHA256SUMS
    done
    echo "contracts/ now holds {{version}}"

# the examples' contract wasm, for deploying
build-examples:
    stellar contract build --profile contract --package ipsa-dispatcher
    stellar contract build --profile contract --package ipsa-poll
    stellar contract build --profile contract --package ipsa-remote-treasury
    stellar contract build --profile contract --package ipsa-payouts
    stellar contract build --profile contract --package ipsa-issuer-console

# the examples' Ethereum contracts: dependencies, formatting, build and tests
solidity:
    cd examples/issuer/token && forge soldeer install && forge fmt --check && forge build && forge test
    cd examples/governance/ethereum-poll && forge soldeer install && forge fmt --check && forge build && forge test

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --locked --workspace --all-targets -- {{lints}}

test:
    cargo test --locked --workspace

audit:
    cargo audit --file Cargo.lock --ignore RUSTSEC-2026-0387

# what crates.io would receive, without sending it
package:
    cargo package --locked --package ipsa

clean:
    cargo clean
