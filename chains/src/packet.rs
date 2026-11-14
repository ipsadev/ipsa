use anyhow::{anyhow, Result};
use soroban_client::xdr::ScVal;

use crate::conversion::{
    scval_as_bytes, scval_as_map, scval_as_string, scval_as_symbol, scval_as_u64, scval_field,
    scval_from_xdr,
};

pub const SEND_PACKET: &str = "send_packet";
pub const WRITE_ACK: &str = "write_ack";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payload {
    pub source_port: String,
    pub dest_port: String,
    pub version: String,
    pub encoding: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub sequence: u64,
    pub source_client: String,
    pub dest_client: String,
    pub timeout_timestamp: u64,
    pub payloads: Vec<Payload>,
}

#[derive(Clone, Debug)]
pub struct PacketEvent {
    pub kind: String,
    pub tx_hash: String,
    pub ledger: u32,
    pub packet: Packet,
    pub acknowledgements: Vec<Vec<u8>>,
}

pub fn event_kind(topics_xdr: &[Vec<u8>]) -> Option<String> {
    scval_from_xdr(topics_xdr.first()?)
        .ok()
        .as_ref()
        .and_then(scval_as_symbol)
}

pub fn decode_packet(value_xdr: &[u8]) -> Result<Packet> {
    let value = scval_from_xdr(value_xdr)?;
    let root = scval_as_map(&value).ok_or_else(|| anyhow!("event body is not a map"))?;
    let packet = scval_field(root, "packet")
        .and_then(scval_as_map)
        .ok_or_else(|| anyhow!("event body has no packet field"))?;

    let sequence = scval_field(packet, "sequence")
        .and_then(scval_as_u64)
        .ok_or_else(|| anyhow!("packet has no sequence"))?;
    let source_client = scval_field(packet, "source_client")
        .and_then(scval_as_string)
        .ok_or_else(|| anyhow!("packet has no source_client"))?;
    let dest_client = scval_field(packet, "dest_client")
        .and_then(scval_as_string)
        .ok_or_else(|| anyhow!("packet has no dest_client"))?;
    let timeout_timestamp = scval_field(packet, "timeout_timestamp")
        .and_then(scval_as_u64)
        .ok_or_else(|| anyhow!("packet has no timeout_timestamp"))?;

    let payloads = match scval_field(packet, "payloads") {
        Some(ScVal::Vec(Some(items))) => items
            .0
            .iter()
            .map(decode_payload)
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(anyhow!("packet has no payloads")),
    };

    Ok(Packet {
        sequence,
        source_client,
        dest_client,
        timeout_timestamp,
        payloads,
    })
}

fn decode_payload(value: &ScVal) -> Result<Payload> {
    let map = scval_as_map(value).ok_or_else(|| anyhow!("payload is not a map"))?;

    let field = |name: &str| {
        scval_field(map, name)
            .and_then(scval_as_string)
            .ok_or_else(|| anyhow!("payload has no {name}"))
    };

    Ok(Payload {
        source_port: field("source_port")?,
        dest_port: field("dest_port")?,
        version: field("version")?,
        encoding: field("encoding")?,
        value: scval_field(map, "value")
            .and_then(scval_as_bytes)
            .ok_or_else(|| anyhow!("payload has no value"))?,
    })
}

pub fn decode_acknowledgements(value_xdr: &[u8]) -> Vec<Vec<u8>> {
    let Ok(value) = scval_from_xdr(value_xdr) else {
        return Vec::new();
    };
    let Some(root) = scval_as_map(&value) else {
        return Vec::new();
    };

    match scval_field(root, "acknowledgements") {
        Some(ScVal::Vec(Some(items))) => items.0.iter().filter_map(scval_as_bytes).collect(),
        _ => Vec::new(),
    }
}
