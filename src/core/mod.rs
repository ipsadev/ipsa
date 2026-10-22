//! the ibc pieces that stellar and its counterparty must compute the same way.

/// ibc v2 commitment paths and hashes, as `ICS24Host.sol` defines them.
pub mod commitment;
/// write, read and check proofs in the format the stellar light client verifies.
pub mod proof;
/// the 64-level sparse merkle tree the stellar ibc router keeps.
pub mod smt;
