pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/gateway_descriptor.bin"));

pub mod proofapi {
    include!(concat!(env!("OUT_DIR"), "/proofapi.rs"));
}

pub mod stellar {
    include!(concat!(env!("OUT_DIR"), "/stellar.gateway.v1.rs"));
}
