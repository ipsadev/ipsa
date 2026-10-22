use ipsa::types::{
    error_acknowledgement, is_error_acknowledgement, OnRecvPacketCallback, Packet, Payload,
};
use soroban_sdk::{vec, Bytes, Env, IntoVal, String, TryFromVal, Val};

fn payload(env: &Env) -> Payload {
    Payload {
        source_port: String::from_str(env, "transfer"),
        dest_port: String::from_str(env, "transfer"),
        version: String::from_str(env, "ics20-1"),
        encoding: String::from_str(env, "application/json"),
        value: Bytes::from_slice(env, b"{}"),
    }
}

#[test]
fn the_error_acknowledgement_is_the_sha256_of_its_preimage() {
    let env = Env::default();
    let expected = env
        .crypto()
        .sha256(&Bytes::from_slice(&env, b"UNIVERSAL_ERROR_ACKNOWLEDGEMENT"))
        .to_array();

    assert_eq!(
        error_acknowledgement(&env),
        Bytes::from_array(&env, &expected)
    );
    assert!(is_error_acknowledgement(&env, &error_acknowledgement(&env)));
    assert!(!is_error_acknowledgement(
        &env,
        &Bytes::from_slice(&env, &[0x01])
    ));
}

#[test]
fn a_packet_survives_a_round_trip_through_a_contract_value() {
    let env = Env::default();
    let packet = Packet {
        sequence: 7,
        source_client: String::from_str(&env, "07-tendermint-0"),
        dest_client: String::from_str(&env, "08-wasm-5"),
        timeout_timestamp: 2_000,
        payloads: vec![&env, payload(&env)],
    };
    let value: Val = packet.clone().into_val(&env);

    assert_eq!(Packet::try_from_val(&env, &value).unwrap(), packet);
}

#[test]
fn a_callback_survives_a_round_trip_through_a_contract_value() {
    let env = Env::default();
    let callback = OnRecvPacketCallback {
        source_client: String::from_str(&env, "08-wasm-5"),
        dest_client: String::from_str(&env, "07-tendermint-0"),
        sequence: 1,
        payload: payload(&env),
    };
    let value: Val = callback.clone().into_val(&env);

    assert_eq!(
        OnRecvPacketCallback::try_from_val(&env, &value).unwrap(),
        callback
    );
}
