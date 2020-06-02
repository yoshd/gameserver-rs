use std::collections::HashMap;
use std::env;

use log::{debug, error, info};
use tokio::sync::mpsc;

pub mod mm {
    tonic::include_proto!("matchmaker");
}

pub mod om {
    tonic::include_proto!("openmatch");
}

pub struct GameFrontend {
    om_frontend_service_client:
        om::frontend_service_client::FrontendServiceClient<tonic::transport::channel::Channel>,
}

impl GameFrontend {
    async fn new(om_frontend_address: String) -> Result<Self, tonic::transport::Error> {
        let client = om::frontend_service_client::FrontendServiceClient::connect(format!(
            "http://{}",
            om_frontend_address
        ))
        .await?;
        Ok(GameFrontend {
            om_frontend_service_client: client,
        })
    }
}

#[tonic::async_trait]
impl mm::frontend_server::Frontend for GameFrontend {
    type CreateMatchStream = mpsc::Receiver<Result<mm::CreateMatchResponse, tonic::Status>>;
    async fn create_match(
        &self,
        request: tonic::Request<mm::CreateMatchRequest>,
    ) -> Result<tonic::Response<Self::CreateMatchStream>, tonic::Status> {
        let (mut tx, rx) = mpsc::channel(1);
        let mut client = self.om_frontend_service_client.clone();
        let player_id = request.into_inner().player_id;
        debug!("requested: {}", player_id);

        let create_ticket_req = om::CreateTicketRequest {
            ticket: Some(om::Ticket {
                id: "".to_string(), // auto gen by open match
                assignment: None,
                search_fields: Some(om::SearchFields {
                    double_args: HashMap::new(),
                    string_args: HashMap::new(),
                    tags: vec![],
                }),
                extensions: std::collections::HashMap::new(),
                create_time: None, // todo
            }),
        };
        let create_ticket_res = client
            .create_ticket(tonic::Request::new(create_ticket_req))
            .await?;
        let ticket = create_ticket_res.into_inner();
        debug!("created ticket: {:?}", ticket);
        let watch_assignments_res = client
            .watch_assignments(tonic::Request::new(om::WatchAssignmentsRequest {
                ticket_id: ticket.id.clone(),
            }))
            .await?;
        let mut inbound = watch_assignments_res.into_inner();
        tokio::spawn(async move {
            while let Ok(assignment_res) = inbound.message().await {
                let assignment = match assignment_res {
                    Some(res) => match res.assignment {
                        Some(assignment) => assignment,
                        None => {
                            error!("empty assignments");
                            if let Err(err) = tx
                                .send(Err(tonic::Status::new(
                                    tonic::Code::Unavailable,
                                    "failed to assign match request",
                                )))
                                .await
                            {
                                error!("failed to send: {:?}", err);
                            }
                            break;
                        }
                    },
                    None => {
                        error!("empty assignments");
                        if let Err(err) = tx
                            .send(Err(tonic::Status::new(
                                tonic::Code::Unavailable,
                                "failed to assign match request",
                            )))
                            .await
                        {
                            error!("failed to send: {:?}", err);
                        }
                        break;
                    }
                };
                let connection = assignment.connection;
                let res = mm::CreateMatchResponse {
                    game_server: Some(mm::GameServer {
                        address: connection,
                    }),
                };
                if let Err(err) = tx.send(Ok(res)).await {
                    error!("failed to send: {:?}", err);
                }
                if let Err(err) = client
                    .delete_ticket(tonic::Request::new(om::DeleteTicketRequest {
                        ticket_id: ticket.id.clone(),
                    }))
                    .await
                {
                    error!("failed to delete ticket: {:?}", err);
                }
                return;
            }
        });
        Ok(tonic::Response::new(rx))
    }
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    info!("start game frontend server");

    let om_frontend_address = env::var("OM_FRONTEND_ADDRESS")
        .unwrap_or("om-frontend.open-match.svc.cluster.local:50504".to_string());
    let address = env::var("ADDRESS")
        .unwrap_or("0.0.0.0:10001".to_string())
        .parse()
        .expect("cannot parse ADDRESS");

    let gf = GameFrontend::new(om_frontend_address).await?;
    let svc = mm::frontend_server::FrontendServer::new(gf);
    tonic::transport::Server::builder()
        .add_service(svc)
        .serve(address)
        .await?;
    Ok(())
}
