use ipsa::{
    app::{IbcApplication, IbcRouterClient},
    types::{OnAcknowledgementPacketCallback, OnRecvPacketCallback, OnTimeoutPacketCallback},
};
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Bytes, Env};

#[contract]
pub struct EchoApplication;

#[contractimpl]
impl IbcApplication for EchoApplication {
    fn on_recv_packet(_env: Env, callback: OnRecvPacketCallback) -> Bytes {
        callback.payload.value
    }

    fn on_acknowledgement_packet(_env: Env, _callback: OnAcknowledgementPacketCallback) {}

    fn on_timeout_packet(_env: Env, _callback: OnTimeoutPacketCallback) {}
}

#[test]
fn an_application_implements_the_router_callbacks() {
    let env = Env::default();
    let id = env.register(EchoApplication, ());
    let client = EchoApplicationClient::new(&env, &id);
    let router = IbcRouterClient::new(&env, &Address::generate(&env));

    assert_eq!(client.address, id);
    assert_ne!(router.address, id);
}
