use soroban_sdk::{contractclient, Address, Bytes, BytesN, Env, String, Vec};

use crate::types::{
    Counterparty, OnAcknowledgementPacketCallback, OnRecvPacketCallback, OnTimeoutPacketCallback,
    Payload,
};

#[contractclient(name = "IbcRouterClient")]
pub trait IbcRouter {
    fn send_packet(
        env: Env,
        source_client_id: String,
        timeout_timestamp: u64,
        payloads: Vec<Payload>,
    ) -> u64;

    fn port_app(env: Env, port_id: String) -> Option<Address>;

    fn port_packets_in_flight(env: Env, port_id: String) -> u64;

    fn counterparty(env: Env, client_id: String) -> Option<Counterparty>;

    fn packet_commitment(env: Env, source_client_id: String, sequence: u64) -> Option<BytesN<32>>;

    fn has_packet_receipt(env: Env, dest_client_id: String, sequence: u64) -> bool;

    fn acknowledgement(env: Env, dest_client_id: String, sequence: u64) -> Option<BytesN<32>>;
}

pub trait IbcApplication {
    fn on_recv_packet(env: Env, callback: OnRecvPacketCallback) -> Bytes;

    fn on_acknowledgement_packet(env: Env, callback: OnAcknowledgementPacketCallback);

    fn on_timeout_packet(env: Env, callback: OnTimeoutPacketCallback);
}
