use ipsa_chains::{
    msg::{ProtoAny, ProtoMessage},
    v2_msgs::{Height, MsgRecvPacket, Packet, Payload, TYPE_URL_RECV_PACKET},
};

#[test]
fn recv_packet_encodes_into_any() {
    let msg = MsgRecvPacket {
        packet: Some(Packet {
            sequence: 1,
            source_client: "07-tendermint-5".to_string(),
            dest_client: "08-wasm-0".to_string(),
            timeout_timestamp: 1_784_946_743,
            payloads: vec![Payload {
                source_port: "transfer".to_string(),
                dest_port: "transfer".to_string(),
                version: "ics20-1".to_string(),
                encoding: "application/json".to_string(),
                value: b"{}".to_vec(),
            }],
        }),
        proof_commitment: b"mock".to_vec(),
        proof_height: Some(Height {
            revision_number: 0,
            revision_height: 3_785_598,
        }),
        signer: "cosmos1mz5kk8dh85ss9dlx5e99cvkl60x92fkmtk6rxj".to_string(),
    };
    let any: ProtoAny = msg.clone().into();

    assert_eq!(any.type_url, TYPE_URL_RECV_PACKET);
    assert_eq!(MsgRecvPacket::decode(any.value.as_slice()).unwrap(), msg);
}

fn top_level_fields(mut bytes: &[u8]) -> std::vec::Vec<u32> {
    fn varint(bytes: &mut &[u8]) -> u64 {
        let mut value = 0u64;
        let mut shift = 0;

        loop {
            let byte = bytes[0];
            *bytes = &bytes[1..];
            value |= u64::from(byte & 0x7f) << shift;

            if byte & 0x80 == 0 {
                return value;
            }

            shift += 7;
        }
    }

    let mut fields = std::vec::Vec::new();

    while !bytes.is_empty() {
        let key = varint(&mut bytes);
        fields.push((key >> 3) as u32);

        match key & 0x7 {
            0 => {
                varint(&mut bytes);
            }
            2 => {
                let length = varint(&mut bytes) as usize;
                bytes = &bytes[length..];
            }
            wire => panic!("unexpected wire type {wire}"),
        }
    }

    fields
}

fn sample_packet() -> Packet {
    Packet {
        sequence: 1,
        source_client: "07-tendermint-0".to_string(),
        dest_client: "08-wasm-0".to_string(),
        timeout_timestamp: 1_784_946_743,
        payloads: vec![],
    }
}

fn sample_height() -> Option<Height> {
    Some(Height {
        revision_number: 0,
        revision_height: 10,
    })
}

const SIGNER: &str = "cosmos1mz5kk8dh85ss9dlx5e99cvkl60x92fkmtk6rxj";

#[test]
fn v2_messages_use_ibc_go_field_numbers() {
    use ipsa_chains::v2_msgs::{Acknowledgement, MsgAcknowledgement, MsgTimeout};

    let recv = MsgRecvPacket {
        packet: Some(sample_packet()),
        proof_commitment: b"proof".to_vec(),
        proof_height: sample_height(),
        signer: SIGNER.to_string(),
    };
    let acknowledgement = MsgAcknowledgement {
        packet: Some(sample_packet()),
        acknowledgement: Some(Acknowledgement {
            app_acknowledgements: vec![b"{\"result\":\"AQ==\"}".to_vec()],
        }),
        proof_acked: b"proof".to_vec(),
        proof_height: sample_height(),
        signer: SIGNER.to_string(),
    };
    let timeout = MsgTimeout {
        packet: Some(sample_packet()),
        proof_unreceived: b"proof".to_vec(),
        proof_height: sample_height(),
        signer: SIGNER.to_string(),
    };

    assert_eq!(top_level_fields(&recv.encode_to_vec()), vec![1, 2, 3, 4]);
    assert_eq!(
        top_level_fields(&acknowledgement.encode_to_vec()),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(
        top_level_fields(&timeout.encode_to_vec()),
        vec![1, 2, 3, 5],
        "ibc-go v11 channel/v2/tx.proto: MsgTimeout.signer is field 5, and 4 is unused"
    );
}
