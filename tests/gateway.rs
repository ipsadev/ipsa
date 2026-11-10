use ipsa::gateway::{
    proofapi::{
        proof_api_service_client::ProofApiServiceClient,
        proof_api_service_server::{ProofApiService, ProofApiServiceServer},
        CreateClientRequest, CreateClientResponse, InfoRequest, InfoResponse, RelayByTxRequest,
        RelayByTxResponse, UpdateClientRequest, UpdateClientResponse,
    },
    stellar::{QueryTreeSiblingsRequest, QueryTreeSiblingsResponse},
};
use prost_gateway::Message;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

struct Echo;

#[tonic::async_trait]
impl ProofApiService for Echo {
    async fn relay_by_tx(
        &self,
        request: Request<RelayByTxRequest>,
    ) -> Result<Response<RelayByTxResponse>, Status> {
        let request = request.into_inner();

        Ok(Response::new(RelayByTxResponse {
            tx: request.source_tx_ids.concat(),
            address: request.dst_chain,
        }))
    }

    async fn create_client(
        &self,
        _: Request<CreateClientRequest>,
    ) -> Result<Response<CreateClientResponse>, Status> {
        Err(Status::unimplemented("create_client"))
    }

    async fn update_client(
        &self,
        _: Request<UpdateClientRequest>,
    ) -> Result<Response<UpdateClientResponse>, Status> {
        Err(Status::unimplemented("update_client"))
    }

    async fn info(&self, _: Request<InfoRequest>) -> Result<Response<InfoResponse>, Status> {
        Err(Status::unimplemented("info"))
    }
}

#[tokio::test]
async fn a_client_relays_through_a_served_proof_api() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(
        Server::builder()
            .add_service(ProofApiServiceServer::new(Echo))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );

    let mut client = ProofApiServiceClient::connect(format!("http://{address}"))
        .await
        .unwrap();

    let response = client
        .relay_by_tx(RelayByTxRequest {
            src_chain: "stellar-testnet".into(),
            dst_chain: "provider".into(),
            source_tx_ids: vec![b"ab".to_vec(), b"cd".to_vec()],
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();

    assert_eq!(response.tx, b"abcd");
    assert_eq!(response.address, "provider");

    let status = client.info(InfoRequest::default()).await.unwrap_err();

    assert_eq!(status.code(), tonic::Code::Unimplemented);
}

#[test]
fn tree_siblings_survive_an_encoding_round_trip() {
    let response = QueryTreeSiblingsResponse {
        ledger: 7,
        root: vec![1; 32],
        value: vec![2; 32],
        siblings: vec![vec![3; 32], vec![4; 32]],
    };

    let decoded = QueryTreeSiblingsResponse::decode(response.encode_to_vec().as_slice()).unwrap();

    assert_eq!(decoded, response);
    assert_eq!(
        QueryTreeSiblingsRequest::decode(
            QueryTreeSiblingsRequest {
                ledger: 7,
                path: vec![9; 32],
            }
            .encode_to_vec()
            .as_slice(),
        )
        .unwrap()
        .ledger,
        7
    );
}

#[test]
fn the_descriptor_set_names_every_service() {
    let descriptors = String::from_utf8_lossy(ipsa::gateway::FILE_DESCRIPTOR_SET);

    assert!(descriptors.contains("ProofApiService"));
    assert!(descriptors.contains("StellarGatewayQuery"));
    assert!(descriptors.contains("StellarGatewayMsg"));
}
