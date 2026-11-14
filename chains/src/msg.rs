use ibc_proto::{
    cosmos::tx::v1beta1::TxBody, google::protobuf::Any, ibc::core::client::v1::Height as RawHeight,
};
pub use ibc_proto::{
    cosmos::tx::v1beta1::TxBody as ProtoTxBody,
    google::protobuf::Any as ProtoAny,
    ibc::{
        core::client::v1::{MsgCreateClient, MsgUpdateClient},
        lightclients::wasm::v1::{
            ClientMessage as WasmClientMessage, ClientState as WasmClientState,
            ConsensusState as WasmConsensusState,
        },
    },
};
use prost::Message;
pub use prost::Message as ProtoMessage;
use soroban_client::xdr::ScVal;

use crate::{
    conversion::{scval_bytes, scval_string, scval_struct, scval_vec},
    packet::{PacketEvent, SEND_PACKET, WRITE_ACK},
    v2_msgs::{
        Acknowledgement, MsgAcknowledgement, MsgRecvPacket, MsgTimeout, Packet as V2Packet, Packet,
        Payload,
    },
};

pub const TYPE_URL_CREATE_CLIENT: &str = "/ibc.core.client.v1.MsgCreateClient";
pub const TYPE_URL_UPDATE_CLIENT: &str = "/ibc.core.client.v1.MsgUpdateClient";
pub const TYPE_URL_WASM_CLIENT_MESSAGE: &str = "/ibc.lightclients.wasm.v1.ClientMessage";
pub const TYPE_URL_WASM_CLIENT_STATE: &str = "/ibc.lightclients.wasm.v1.ClientState";
pub const TYPE_URL_WASM_CONSENSUS_STATE: &str = "/ibc.lightclients.wasm.v1.ConsensusState";

pub fn wasm_client_state(client_state: Vec<u8>, checksum: Vec<u8>, latest_height: u64) -> Any {
    let wrapped = WasmClientState {
        data: client_state,
        checksum,
        latest_height: Some(RawHeight {
            revision_number: 0,
            revision_height: latest_height,
        }),
    };

    Any {
        type_url: TYPE_URL_WASM_CLIENT_STATE.to_string(),
        value: wrapped.encode_to_vec(),
    }
}

pub fn wasm_consensus_state(consensus_state: Vec<u8>) -> Any {
    Any {
        type_url: TYPE_URL_WASM_CONSENSUS_STATE.to_string(),
        value: WasmConsensusState {
            data: consensus_state,
        }
        .encode_to_vec(),
    }
}

pub fn create_client_msg(client_state: Any, consensus_state: Any, signer: &str) -> Any {
    let msg = MsgCreateClient {
        client_state: Some(client_state),
        consensus_state: Some(consensus_state),
        signer: signer.to_string(),
    };

    Any {
        type_url: TYPE_URL_CREATE_CLIENT.to_string(),
        value: msg.encode_to_vec(),
    }
}

pub fn wasm_client_message(header: Vec<u8>) -> Any {
    Any {
        type_url: TYPE_URL_WASM_CLIENT_MESSAGE.to_string(),
        value: WasmClientMessage { data: header }.encode_to_vec(),
    }
}

pub fn update_client_msg(client_id: &str, client_message: Any, signer: &str) -> Any {
    let msg = MsgUpdateClient {
        client_id: client_id.to_string(),
        client_message: Some(client_message),
        signer: signer.to_string(),
    };

    Any {
        type_url: TYPE_URL_UPDATE_CLIENT.to_string(),
        value: msg.encode_to_vec(),
    }
}

pub fn to_proto_packet(packet: &crate::packet::Packet) -> Packet {
    Packet {
        sequence: packet.sequence,
        source_client: packet.source_client.clone(),
        dest_client: packet.dest_client.clone(),
        timeout_timestamp: packet.timeout_timestamp,
        payloads: packet
            .payloads
            .iter()
            .map(|payload| Payload {
                source_port: payload.source_port.clone(),
                dest_port: payload.dest_port.clone(),
                version: payload.version.clone(),
                encoding: payload.encoding.clone(),
                value: payload.value.clone(),
            })
            .collect(),
    }
}

pub fn recv_msg(event: &PacketEvent, signer: &str) -> MsgRecvPacket {
    MsgRecvPacket {
        packet: Some(to_proto_packet(&event.packet)),
        proof_commitment: Vec::new(),
        proof_height: None,
        signer: signer.to_string(),
    }
}

pub fn ack_msg(event: &PacketEvent, signer: &str) -> MsgAcknowledgement {
    MsgAcknowledgement {
        packet: Some(to_proto_packet(&event.packet)),
        acknowledgement: Some(Acknowledgement {
            app_acknowledgements: event.acknowledgements.clone(),
        }),
        proof_acked: Vec::new(),
        proof_height: None,
        signer: signer.to_string(),
    }
}

pub fn recv_msgs(events: &[PacketEvent], signer: &str) -> Vec<MsgRecvPacket> {
    events
        .iter()
        .filter(|event| event.kind == SEND_PACKET)
        .map(|event| recv_msg(event, signer))
        .collect()
}

pub fn ack_msgs(events: &[PacketEvent], signer: &str) -> Vec<MsgAcknowledgement> {
    events
        .iter()
        .filter(|event| event.kind == WRITE_ACK)
        .map(|event| ack_msg(event, signer))
        .collect()
}

pub fn encode_msgs(messages: Vec<Any>) -> Vec<u8> {
    TxBody {
        messages,
        ..Default::default()
    }
    .encode_to_vec()
}

pub fn encode_tx_body(
    update_client: Option<Any>,
    recv: Vec<MsgRecvPacket>,
    ack: Vec<MsgAcknowledgement>,
) -> Vec<u8> {
    encode_tx_body_with_timeouts(update_client, recv, ack, Vec::new())
}

pub fn encode_tx_body_with_timeouts(
    update_client: Option<Any>,
    recv: Vec<MsgRecvPacket>,
    ack: Vec<MsgAcknowledgement>,
    timeout: Vec<MsgTimeout>,
) -> Vec<u8> {
    encode_msgs(
        update_client
            .into_iter()
            .chain(recv.into_iter().map(Any::from))
            .chain(ack.into_iter().map(Any::from))
            .chain(timeout.into_iter().map(Any::from))
            .collect(),
    )
}

pub fn encode_packet(packet: &V2Packet) -> Vec<u8> {
    packet.encode_to_vec()
}

pub fn decode_packet(bytes: &[u8]) -> Result<V2Packet, prost::DecodeError> {
    V2Packet::decode(bytes)
}

pub fn decode_acknowledgement(bytes: &[u8]) -> Result<Vec<Vec<u8>>, prost::DecodeError> {
    Acknowledgement::decode(bytes).map(|a| a.app_acknowledgements)
}

pub fn merkle_proof_from_ops(ops: Vec<Vec<u8>>) -> Result<Vec<u8>, prost::DecodeError> {
    let mut proofs = Vec::with_capacity(ops.len());
    for op in ops {
        proofs.push(ics23::CommitmentProof::decode(op.as_slice())?);
    }
    Ok(ibc_proto::ibc::core::commitment::v1::MerkleProof { proofs }.encode_to_vec())
}

pub fn packet_to_scval(packet: &V2Packet) -> anyhow::Result<ScVal> {
    let mut payloads = Vec::with_capacity(packet.payloads.len());
    for payload in &packet.payloads {
        payloads.push(scval_struct(vec![
            ("dest_port", scval_string(&payload.dest_port)?),
            ("encoding", scval_string(&payload.encoding)?),
            ("source_port", scval_string(&payload.source_port)?),
            ("value", scval_bytes(&payload.value)?),
            ("version", scval_string(&payload.version)?),
        ])?);
    }

    scval_struct(vec![
        ("dest_client", scval_string(&packet.dest_client)?),
        ("payloads", scval_vec(payloads)?),
        ("sequence", ScVal::U64(packet.sequence)),
        ("source_client", scval_string(&packet.source_client)?),
        ("timeout_timestamp", ScVal::U64(packet.timeout_timestamp)),
    ])
}
