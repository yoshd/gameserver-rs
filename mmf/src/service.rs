use std::collections::HashMap;
use std::env;

use futures::StreamExt;
use log::{debug, error, info};
use tokio::sync::mpsc;
use uuid::Uuid;

pub mod om {
    tonic::include_proto!("openmatch");
}

pub struct MatchMakingFunctionService {
    match_function_name: String,
    num_matching_members: usize,
    om_mml_client: om::query_service_client::QueryServiceClient<tonic::transport::channel::Channel>,
}

impl MatchMakingFunctionService {
    async fn new(
        match_function_name: String,
        num_matching_members: usize,
        om_query_address: String,
    ) -> Result<Self, tonic::transport::Error> {
        let client = om::query_service_client::QueryServiceClient::connect(format!(
            "http://{}",
            om_query_address
        ))
        .await?;
        Ok(MatchMakingFunctionService {
            match_function_name,
            num_matching_members,
            om_mml_client: client,
        })
    }
}

#[tonic::async_trait]
impl om::match_function_server::MatchFunction for MatchMakingFunctionService {
    type RunStream = mpsc::Receiver<Result<om::RunResponse, tonic::Status>>;
    async fn run(
        &self,
        request: tonic::Request<om::RunRequest>,
    ) -> Result<tonic::Response<Self::RunStream>, tonic::Status> {
        let (mut tx, rx) = mpsc::channel(1);
        let profile = request.into_inner().profile.ok_or(tonic::Status::new(
            tonic::Code::InvalidArgument,
            "profile is not specified",
        ))?;
        debug!("requested. profile: {:?}", profile);
        let mut om_mml_client = self.om_mml_client.clone();
        let num_matching_members = self.num_matching_members;
        let match_function_name = self.match_function_name.clone();

        tokio::spawn(async move {
            debug!("requested. profile: {:?}", profile);
            for pool in profile.pools {
                let req = om::QueryTicketsRequest {
                    pool: Some(pool.clone()),
                };
                let stream = match om_mml_client
                    .query_tickets(tonic::Request::new(req))
                    .await
                    .map_err(|err| tonic::Status::new(tonic::Code::Unavailable, err.to_string()))
                {
                    Ok(stream) => stream.into_inner(),
                    Err(err) => {
                        if let Err(err) = tx.send(Err(err)).await {
                            error!("{:?}", err);
                        }
                        return;
                    }
                };

                let mut all_tickets = Vec::new();
                futures::pin_mut!(stream);
                while let Some(res) = stream.next().await {
                    match res {
                        Ok(mut res) => all_tickets.append(&mut res.tickets),
                        Err(err) => {
                            if let Err(err) = tx
                                .send(Err(tonic::Status::new(
                                    tonic::Code::Unavailable,
                                    err.to_string(),
                                )))
                                .await
                            {
                                error!("{:?}", err);
                            }
                            return;
                        }
                    }
                }
                debug!("all tickets: {:?}", all_tickets);
                while all_tickets.len() >= num_matching_members {
                    let tickets = all_tickets[0..num_matching_members].to_vec();
                    all_tickets = all_tickets[num_matching_members..].to_vec();
                    let result = om::RunResponse {
                        proposal: Some(om::Match {
                            match_id: Uuid::new_v4().to_string(),
                            match_profile: profile.name.clone(),
                            match_function: match_function_name.clone(),
                            tickets: tickets,
                            extensions: HashMap::new(),
                        }),
                    };
                    debug!("result: {:?}", result);
                    if let Err(err) = tx.send(Ok(result)).await.map_err(|err| {
                        tonic::Status::new(tonic::Code::Unavailable, err.to_string())
                    }) {
                        error!("{:?}", err);
                    }
                }
            }
        });
        Ok(tonic::Response::new(rx))
    }
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    info!("start server");

    let match_function_name = env::var("MATCH_FUNCTION_NAME").unwrap_or("basic".to_string());
    let num_matching_members: usize = env::var("NUM_MATCHING_MEMBERS")
        .unwrap_or("2".to_string())
        .parse()
        .expect("cannot parse NUM_MATCHING_MEMBERS");
    let om_query_address = env::var("OM_QUERY_ADDRESS")
        .unwrap_or("om-query.open-match.svc.cluster.local:50503".to_string());
    let address = env::var("ADDRESS")
        .unwrap_or("0.0.0.0:50502".to_string())
        .parse()
        .expect("cannot parse ADDRESS");

    let mmf = MatchMakingFunctionService::new(
        match_function_name,
        num_matching_members,
        om_query_address,
    )
    .await?;

    let svc = om::match_function_server::MatchFunctionServer::new(mmf);
    tonic::transport::Server::builder()
        .add_service(svc)
        .serve(address)
        .await?;
    Ok(())
}
