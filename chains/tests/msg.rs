use ipsa_chains::{
    msg::{
        ack_msgs, encode_tx_body, recv_msgs, update_client_msg, wasm_client_message,
        MsgUpdateClient, ProtoAny, ProtoMessage, ProtoTxBody, WasmClientMessage,
        TYPE_URL_UPDATE_CLIENT, TYPE_URL_WASM_CLIENT_MESSAGE,
    },
    packet::{Packet, PacketEvent, Payload, SEND_PACKET, WRITE_ACK},
    v2_msgs::{Acknowledgement, TYPE_URL_RECV_PACKET},
};

fn reference_event() -> PacketEvent {
    PacketEvent {
        kind: SEND_PACKET.to_string(),
        tx_hash: "2c20a54f9381f365ccd40356732da798fa77e3e91826ce0b5322764989167589".to_string(),
        ledger: 3_785_597,
        acknowledgements: Vec::new(),
        packet: Packet {
            sequence: 1,
            source_client: "07-tendermint-5".to_string(),
            dest_client: "08-wasm-0".to_string(),
            timeout_timestamp: 1_784_946_743,
            payloads: vec![Payload {
                source_port: "transfer".to_string(),
                dest_port: "transfer".to_string(),
                version: "ics20-1".to_string(),
                encoding: "application/json".to_string(),
                value: br#"{"denom":"XLM","amount":"1000"}"#.to_vec(),
            }],
        },
    }
}

#[test]
fn builds_a_recv_msg_matching_the_hermes_reference() {
    let signer = "cosmos1mz5kk8dh85ss9dlx5e99cvkl60x92fkmtk6rxj";
    let msgs = recv_msgs(&[reference_event()], signer);

    assert_eq!(msgs.len(), 1);

    let packet = msgs[0].packet.as_ref().unwrap();

    assert_eq!(packet.sequence, 1);
    assert_eq!(packet.source_client, "07-tendermint-5");
    assert_eq!(packet.dest_client, "08-wasm-0");
    assert_eq!(packet.timeout_timestamp, 1_784_946_743);
    assert_eq!(packet.payloads.len(), 1);
    assert_eq!(packet.payloads[0].version, "ics20-1");
    assert_eq!(packet.payloads[0].encoding, "application/json");
    assert_eq!(msgs[0].signer, signer);
}

#[test]
fn messages_are_built_without_proofs() {
    let recv = recv_msgs(&[reference_event()], "cosmos1abc");

    assert!(recv[0].proof_commitment.is_empty());
    assert_eq!(recv[0].proof_height, None);
}

#[test]
fn ack_msgs_select_write_ack_events_only() {
    let send = reference_event();
    let mut write_ack = reference_event();

    write_ack.kind = WRITE_ACK.to_string();
    write_ack.acknowledgements = vec![b"\x01".to_vec()];

    let acks = ack_msgs(&[send, write_ack], "cosmos1abc");

    assert_eq!(acks.len(), 1);
    assert_eq!(
        acks[0].acknowledgement,
        Some(Acknowledgement {
            app_acknowledgements: vec![b"\x01".to_vec()],
        })
    );
    assert!(acks[0].proof_acked.is_empty());
}

#[test]
fn a_header_is_wrapped_for_the_wasm_client() {
    let any = wasm_client_message(b"header-bytes".to_vec());

    assert_eq!(any.type_url, TYPE_URL_WASM_CLIENT_MESSAGE);

    let inner = WasmClientMessage::decode(any.value.as_slice()).unwrap();

    assert_eq!(inner.data, b"header-bytes");
}

#[test]
fn an_update_carries_the_client_id_and_wrapped_header() {
    let any = update_client_msg(
        "08-wasm-0",
        wasm_client_message(b"header-bytes".to_vec()),
        "cosmos1abc",
    );

    assert_eq!(any.type_url, TYPE_URL_UPDATE_CLIENT);

    let msg = MsgUpdateClient::decode(any.value.as_slice()).unwrap();

    assert_eq!(msg.client_id, "08-wasm-0");
    assert_eq!(msg.signer, "cosmos1abc");

    let carried = msg.client_message.unwrap();

    assert_eq!(carried.type_url, TYPE_URL_WASM_CLIENT_MESSAGE);
    assert_eq!(
        WasmClientMessage::decode(carried.value.as_slice())
            .unwrap()
            .data,
        b"header-bytes"
    );
}

#[test]
fn tx_body_puts_update_client_first() {
    let update = ProtoAny {
        type_url: "/ibc.core.client.v1.MsgUpdateClient".to_string(),
        value: Vec::new(),
    };
    let recv = recv_msgs(&[reference_event()], "cosmos1abc");
    let bytes = encode_tx_body(Some(update), recv, Vec::new());
    let body = ProtoTxBody::decode(bytes.as_slice()).unwrap();

    assert_eq!(body.messages.len(), 2);
    assert_eq!(
        body.messages[0].type_url,
        "/ibc.core.client.v1.MsgUpdateClient"
    );
    assert_eq!(body.messages[1].type_url, TYPE_URL_RECV_PACKET);
}
