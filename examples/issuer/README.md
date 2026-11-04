# Issuer

How a Stellar asset issuer takes their asset to Ethereum without a bridge, and keeps control of it there from Stellar.

The asset itself does not change. A holder sends it through the Stellar transfer app, which escrows it on Stellar, and the Ethereum transfer app mints the issuer's own token instead of a generic voucher. Sending it back burns it on Ethereum and releases the escrow on Stellar, so supply on Ethereum can never exceed what is escrowed. Reserves, compliance and redemption stay on Stellar.

The example has two contracts:

- `console`: a Soroban contract the issuer owns on Stellar. It freezes and unfreezes holders and pauses and unpauses the token on Ethereum, by sending the Solidity call through GMP. It records each action as pending, executed, rejected or timed out, from the GMP app's callbacks.
- `token`: the issuer's ERC-20 on Ethereum, a Foundry project. Only `ICS20Transfer` mints and burns it, and only its admin freezes, pauses or hands over admin. The admin is the console's GMP account on Ethereum, so only the console's owner on Stellar controls it.

## How the pieces connect

1. The issuer registers their asset with the Stellar transfer app as a local asset under a denomination, for example `BRLA`, at its Stellar Asset Contract address.
2. The issuer deploys `console` on Stellar, with the GMP app, the Stellar client on the Ethereum side as seen from Stellar, and the token address once it exists.
3. The issuer derives the console's account on Ethereum, which `ICS27GMP` creates on the first call, and deploys `token` with it as admin.
4. The token is registered with `ICS20Transfer` for the denomination the asset carries on Ethereum, `transfer/<Stellar client id on Ethereum>/<denomination>`, through `setCustomERC20`. That needs `ERC20_CUSTOMIZER_ROLE` on the transfer app's access manager.
5. The console's owner on Stellar calls `freeze`, `unfreeze`, `pause` or `unpause`, and reads the outcome with `record(sequence)` once the relayer brings the acknowledgement back.

A frozen holder can neither send nor receive, so a transfer to them from Stellar is refused on Ethereum and refunded on Stellar. A paused token refuses every movement, including arriving transfers, which are refunded the same way.

## Deploying on testnet

The console, from this repository:

```sh
just build-examples

stellar contract deploy --source admin --network testnet \
  --wasm target/wasm32v1-none/contract/ipsa_issuer_console.wasm \
  -- --owner <issuer account> --gmp $GMP_CONTRACT_ADDRESS --client_id $STELLAR_EVM_CLIENT_ID --token 0x0000000000000000000000000000000000000000
```

The console's account on Ethereum, with the ipsa CLI:

```sh
ipsa tx gmp account --evm --sender <console address>
```

The token, from `examples/issuer/token`, with `EVM_ICS20_TRANSFER` set to the Ethereum transfer app and an account holding `ERC20_CUSTOMIZER_ROLE`:

```sh
forge soldeer install

ISSUER_ADMIN=<console account on Ethereum> \
ISSUER_DENOM=transfer/$EVM_STELLAR_CLIENT_ID/BRLA \
ISSUER_TOKEN_NAME="Brazilian Real" \
ISSUER_TOKEN_SYMBOL=BRLA \
forge script script/DeployIssuerToken.s.sol --rpc-url $EVM_RPC_URL --private-key $EVM_PRIVATE_KEY --broadcast --skip-simulation
```

Then point the console at the token:

```sh
stellar contract invoke --source <issuer account> --network testnet --id <console address> -- set_token --token <token address>
```

`--skip-simulation` is there because Sepolia prices state creation above what Forge estimates. `ISSUER_TOKEN_DECIMALS` defaults to 7, the decimals of every Stellar asset, so amounts mean the same on both sides.

## What the issuer must not do

The escrow is the backing. An issuer whose asset has clawback enabled must never claw back from the Stellar transfer app's balance, and must never freeze the transfer app's escrow on Ethereum: either leaves tokens on the other chain without backing.

## What this example leaves out

- Caps per chain and the supply view: the Stellar transfer app has daily limits per asset, but setting them is still the transfer app owner's call, not the issuer's.
- An issuer whose asset started on Ethereum and comes to Stellar: the same idea the other way round, with the issuer's Stellar Asset Contract registered as the voucher and the transfer app as its admin.
- Urgency: an action from Stellar reaches Ethereum within minutes, but its outcome only comes back after Ethereum finality, about an hour later.

## Tests

```sh
cargo test -p ipsa-issuer-console
cd examples/issuer/token && forge test
```

The console is tested against the router and GMP app wasm from an `ipsa-contracts` release. The calldata it builds is checked byte for byte against `cast calldata`, and the acknowledgements it reads are ABI-encoded as `ICS27GMP` writes them.
