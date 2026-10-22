//! building blocks for ibc v2 on stellar, each behind a feature of the same name.
//!
//! - [`core`]: commitment paths, the sparse merkle tree and its ics-23 proofs.
//! - [`types`]: the packet, payload and callback types of the soroban router.
//! - [`app`]: a client for the router, and the interface an ibc application implements.
//! - [`gmp`]: the ics-27 gmp types, client, callbacks and remote account derivation.
//! - [`ics20`]: the ics-20 types, client and callbacks.

#![no_std]

#[cfg(feature = "core")]
extern crate alloc;

#[cfg(feature = "app")]
pub mod app;
#[cfg(feature = "core")]
pub mod core;
#[cfg(feature = "gmp")]
pub mod gmp;
#[cfg(feature = "ics20")]
pub mod ics20;
#[cfg(feature = "types")]
pub mod types;
