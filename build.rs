fn main() {
    #[cfg(feature = "gateway")]
    gateway();
}

#[cfg(feature = "gateway")]
fn gateway() {
    let protos = ["proto/proofapi.proto", "proto/stellar_gateway.proto"];

    for proto in &protos {
        println!("cargo:rerun-if-changed={proto}");
    }

    let descriptors = protox::compile(protos, ["proto/"]).expect("compiling the gateway protos");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));

    std::fs::write(
        out.join("gateway_descriptor.bin"),
        prost_gateway::Message::encode_to_vec(&descriptors),
    )
    .expect("writing the gateway descriptor set");

    let mut config = prost_build::Config::new();
    config.prost_path("::prost_gateway");

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_fds_with_config(descriptors, config)
        .expect("generating the gateway clients and servers");
}
