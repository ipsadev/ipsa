# `ipsa-payouts` — paying people on other chains

An example of a Soroban contract sending tokens to another chain as itself, through the ipsa transfer app.

`send` is owner-only. It looks up the token for the denom with `asset`, authorises the transfer app to take the amount into escrow, and calls `initiate_transfer` with this contract as the sender. It records `Pending` under the source client and sequence.

The escrow is a token `transfer` from this contract to the transfer app, one call below `initiate_transfer`, so the contract authorises it first with `authorize_as_current_contract`. Without that, the transfer fails with an authorisation error.

The transfer app reports the outcome through `TransferSender`, after any refund has landed:

- `on_transfer_acknowledgement` records `Acknowledged`, with `success` false when the other chain rejected the transfer and the tokens came back.
- `on_transfer_timeout` records `TimedOut`; the tokens are back.

Both refuse a caller other than the transfer app this contract was set up with, and require its authorisation. `outcome(source_client_id, sequence)` reads what was recorded.

The example sends local assets, which are escrowed. Sending a voucher burns it instead, and the contract would authorise a `burn` rather than a `transfer`.
