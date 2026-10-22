# Crates for building on ipsa: IBC v2 types, the router client, GMP and ICS-20

lints := "-D warnings -A clippy::manual_is_multiple_of -A clippy::too_many_arguments -A clippy::result_large_err"

default:
    @just --list

contracts_repository := "ipsadev/ipsa-contracts"
contracts_version := env_var_or_default("CONTRACTS_VERSION", "v0.1.2")

# everything CI runs, in CI's order
ci: fmt-check contracts lint test audit

# the ipsa contract wasm the examples' tests import: a release, or a local build from CONTRACTS_WASM_DIR
contracts:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p contracts
    if [ -n "${CONTRACTS_WASM_DIR:-}" ]; then
        cp "$CONTRACTS_WASM_DIR"/*.wasm contracts/
        echo "copied the local build in $CONTRACTS_WASM_DIR"
        exit 0
    fi
    gh release download "{{contracts_version}}" --repo "{{contracts_repository}}" \
        --pattern '*.wasm' --pattern SHA256SUMS --dir contracts --clobber
    cd contracts && shasum -a 256 -c SHA256SUMS

# the examples' contract wasm, for deploying
build-examples:
    stellar contract build --profile contract --package ipsa-gmp-caller

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --locked --workspace --all-targets -- {{lints}}

test:
    cargo test --locked --workspace

audit:
    cargo audit --file Cargo.lock

# what crates.io would receive, without sending it
package:
    cargo package --locked --package ipsa

clean:
    cargo clean
