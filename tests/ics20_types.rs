use ipsa::ics20::{AssetKind, FungibleTokenPacketData, Token, TransferAcknowledged};
use soroban_sdk::{Bytes, Env, IntoVal, String, TryFromVal, Val};

#[test]
fn packet_data_survives_a_round_trip_through_a_contract_value() {
    let env = Env::default();
    let packet = FungibleTokenPacketData {
        token: Token {
            denom: String::from_str(&env, "XLM"),
            amount: 1_000,
        },
        sender: String::from_str(
            &env,
            "GAPCJ2GXGIY2VRLK6W4DDY443R3JDMJ3K5LQBMUJL4LDSX34MXQMSHVD",
        ),
        receiver: String::from_str(&env, "cosmos1abc"),
        memo: String::from_str(&env, ""),
    };
    let value: Val = packet.clone().into_val(&env);

    assert_eq!(
        FungibleTokenPacketData::try_from_val(&env, &value).unwrap(),
        packet
    );
}

#[test]
fn an_acknowledgement_report_survives_a_round_trip_through_a_contract_value() {
    let env = Env::default();
    let report = TransferAcknowledged {
        source_client: String::from_str(&env, "07-tendermint-0"),
        dest_client: String::from_str(&env, "08-wasm-5"),
        sequence: 3,
        success: false,
        acknowledgement: Bytes::from_slice(&env, &[0xff]),
    };
    let value: Val = report.clone().into_val(&env);

    assert_eq!(
        TransferAcknowledged::try_from_val(&env, &value).unwrap(),
        report
    );
}

#[test]
fn asset_kinds_are_distinct_values() {
    let env = Env::default();
    let local: Val = AssetKind::Local.into_val(&env);

    assert_eq!(
        AssetKind::try_from_val(&env, &local).unwrap(),
        AssetKind::Local
    );
    assert_ne!(AssetKind::Local, AssetKind::Voucher);
}
