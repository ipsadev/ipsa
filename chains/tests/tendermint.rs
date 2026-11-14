use ibc_proto::ibc::lightclients::tendermint::v1::Header as RawTmHeader;
use ipsa_chains::{
    conversion::scval_from_xdr, soroban::tendermint_header_to_soroban_xdr, tendermint::header_proto,
};
use prost::Message;
use serde_json::Value;
use soroban_client::xdr::ScVal;

const TRUSTED_HEIGHT: u64 = 19_532_136;

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/hub-validator-set-change/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );

    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn header() -> Vec<u8> {
    header_proto(
        &fixture("commit"),
        &fixture("validators"),
        &fixture("trusted_validators"),
        TRUSTED_HEIGHT,
        0,
    )
    .unwrap()
}

fn hex_addresses(validators: &Value) -> Vec<Vec<u8>> {
    validators["validators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| hex::decode(v["address"].as_str().unwrap()).unwrap())
        .collect()
}

#[test]
fn a_header_carries_the_validators_the_client_trusts_apart_from_its_own() {
    let header = RawTmHeader::decode(header().as_slice()).unwrap();

    let mut own: Vec<Vec<u8>> = header
        .validator_set
        .as_ref()
        .unwrap()
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();
    let mut trusted: Vec<Vec<u8>> = header
        .trusted_validators
        .as_ref()
        .unwrap()
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();
    let mut expected_own = hex_addresses(&fixture("validators"));
    let mut expected_trusted = hex_addresses(&fixture("trusted_validators"));

    own.sort();
    trusted.sort();
    expected_own.sort();
    expected_trusted.sort();

    assert_eq!(own, expected_own);
    assert_eq!(trusted, expected_trusted);
    assert_ne!(own, trusted);
    assert_eq!(
        header.trusted_height.unwrap().revision_height,
        TRUSTED_HEIGHT
    );
}

#[test]
fn the_soroban_header_passes_the_trusted_validators_to_the_light_client() {
    let proto = header();
    let raw = RawTmHeader::decode(proto.as_slice()).unwrap();
    let expected = raw.trusted_validators.unwrap().encode_to_vec();

    let ScVal::Map(Some(map)) =
        scval_from_xdr(&tendermint_header_to_soroban_xdr(&proto).unwrap()).unwrap()
    else {
        panic!("the soroban header is not a map");
    };

    let field = map
        .iter()
        .find(|entry| matches!(&entry.key, ScVal::Symbol(name) if name.to_string() == "trusted_validator_set_bytes"))
        .expect("the soroban header has no trusted_validator_set_bytes");

    let ScVal::Bytes(bytes) = &field.val else {
        panic!("trusted_validator_set_bytes is not bytes");
    };

    assert_eq!(bytes.to_vec(), expected);
}
