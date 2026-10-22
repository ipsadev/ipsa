# `ipsa-remote-treasury` — a treasury on Stellar governed from another chain

An example of the receiving side of ICS-27 GMP on Soroban.

A call from another chain reaches this contract from an account the GMP app derives from the client the packet arrived on, the remote sender and the salt. The constructor computes that account with `ipsa::gmp::remote_account` and stores it, and `set_limit` requires its authorisation. A call from any other sender, on any other client or with any other salt, comes from a different account and is refused; the GMP app answers it with the universal error acknowledgement.

The account is computed once, at setup. The contract cannot ask the GMP app during the call, because Soroban does not let a contract be re-entered while it runs, and the GMP app is still running when this contract executes.

`remote_account()` returns the stored account, and the tests check it matches the GMP app's own `account_address`.
