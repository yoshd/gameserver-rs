use std::collections::HashMap;
use std::env;
use std::time::Duration;

use async_trait::async_trait;
use http::header::HeaderValue;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::time;

use kube::{
    api::{PostParams, RawApi},
    client::APIClient,
    config,
};

pub mod om {
    include!(concat!(env!("OUT_DIR"), "/openmatch.rs"));
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocateResponse {
    kind: String,
    api_version: String,
    status: Status,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    state: String,
    ports: Option<Vec<Port>>,
    address: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    name: String,
    port: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocateRequest {
    api_version: String,
    kind: String,
    spec: Spec,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    required: Required,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Required {
    match_labels: HashMap<String, String>,
}

#[async_trait]
pub trait GameServerAllocationClient {
    async fn allocate(&self) -> anyhow::Result<AllocateResponse>;
}

pub struct AgonesGameServerAllocationClient {
    k8s_api_client: APIClient,
    k8s_namespace: String,
}

impl AgonesGameServerAllocationClient {
    fn new(k8s_namespace: String) -> anyhow::Result<Self> {
        let config = config::incluster_config()?;
        let client = APIClient::new(config);
        Ok(AgonesGameServerAllocationClient {
            k8s_api_client: client,
            k8s_namespace: k8s_namespace,
        })
    }
}

#[async_trait]
impl GameServerAllocationClient for AgonesGameServerAllocationClient {
    async fn allocate(&self) -> anyhow::Result<AllocateResponse> {
        let pp = PostParams::default();
        let custom_resource = RawApi::customResource("gameserverallocations")
            .version("v1")
            .group("allocation.agones.dev")
            .within(&self.k8s_namespace);
        let mut labels = HashMap::new();
        labels.insert("agones.dev/fleet".to_string(), "gameserver".to_string());

        let req = AllocateRequest {
            api_version: "allocation.agones.dev/v1".to_string(),
            kind: "GameServerAllocation".to_string(),
            spec: Spec {
                required: Required {
                    match_labels: labels,
                },
            },
        };
        let mut request = custom_resource.create(&pp, serde_json::to_vec(&req)?)?;
        request
            .headers_mut()
            .insert("Content-Type", HeaderValue::from_static("application/json"));

        self.k8s_api_client
            .request::<AllocateResponse>(request)
            .await
            .map_err(|err| err.into())
    }
}

#[async_trait]
pub trait Director {
    async fn assign(&mut self) -> anyhow::Result<()>;
}

pub struct OpenMatchDirector<T>
where
    T: GameServerAllocationClient,
{
    gs_alloc_client: T,
    om_backend_client: om::backend_service_client::BackendServiceClient<tonic::transport::Channel>,
    k8s_namespace: String,
}

impl<T> OpenMatchDirector<T>
where
    T: GameServerAllocationClient,
{
    async fn new(
        gs_alloc_client: T,
        om_backend_address: String,
        k8s_namespace: String,
    ) -> anyhow::Result<Self> {
        let client = om::backend_service_client::BackendServiceClient::connect(format!(
            "http://{}",
            om_backend_address
        ))
        .await?;
        Ok(OpenMatchDirector {
            gs_alloc_client: gs_alloc_client,
            om_backend_client: client,
            k8s_namespace: k8s_namespace,
        })
    }
}

#[async_trait]
impl<T> Director for OpenMatchDirector<T>
where
    T: GameServerAllocationClient + Sync + Send,
{
    async fn assign(&mut self) -> anyhow::Result<()> {
        let req = om::FetchMatchesRequest {
            config: Some(om::FunctionConfig {
                host: format!("mmf.{}.svc.cluster.local", self.k8s_namespace),
                port: 50502,
                r#type: om::function_config::Type::Grpc as i32,
            }),
            profile: Some(om::MatchProfile {
                name: "default".to_string(),
                pools: vec![om::Pool {
                    name: "default".to_string(),
                    double_range_filters: vec![],
                    string_equals_filters: vec![],
                    tag_present_filters: vec![],
                }],
                extensions: HashMap::new(),
            }),
        };
        let mut stream = self
            .om_backend_client
            .fetch_matches(tonic::Request::new(req))
            .await?
            .into_inner();
        while let Some(res) = stream.message().await? {
            match res.r#match {
                Some(m) => {
                    let mut ticket_ids = Vec::with_capacity(m.tickets.len());
                    for ticket in m.tickets {
                        ticket_ids.push(ticket.id.clone());
                    }
                    debug!("ticket_ids: {:?}", ticket_ids);

                    let result = self.gs_alloc_client.allocate().await?;
                    if &*result.status.state != "Allocated" {
                        return Err(anyhow::anyhow!(
                            "failed to allocate game server: {}",
                            result.status.state
                        ));
                    }

                    let host = result
                        .status
                        .address
                        .ok_or(anyhow::anyhow!("host is empty"))?;
                    let port = result
                        .status
                        .ports
                        .ok_or(anyhow::anyhow!("port is empty"))?
                        .first()
                        .ok_or(anyhow::anyhow!("port is empty"))?
                        .port
                        .to_string();
                    self.om_backend_client
                        .assign_tickets(om::AssignTicketsRequest {
                            ticket_ids: ticket_ids,
                            assignment: Some(om::Assignment {
                                connection: host + ":" + &port,
                                extensions: HashMap::new(),
                            }),
                        })
                        .await?;
                }
                None => return Err(anyhow::anyhow!("match not found")),
            }
        }
        Ok(())
    }
}

pub struct Worker<T>
where
    T: Director,
{
    director: T,
}

impl<T> Worker<T>
where
    T: Director,
{
    fn new(director: T) -> anyhow::Result<Self> {
        Ok(Worker { director: director })
    }

    async fn run(&mut self) {
        let mut interval = time::interval(Duration::from_millis(2000));
        loop {
            if let Err(err) = self.director.assign().await {
                debug!("{:?}", err);
                interval.tick().await;
            }
        }
    }
}

pub async fn run_worker() -> anyhow::Result<()> {
    let om_backend_address = env::var("OM_BACKEND_ADDRESS")
        .unwrap_or("om-backend.open-match.svc.cluster.local:50505".to_string());
    let gameserver_namespace = env::var("GAMESERVER_NAMESPACE").unwrap_or("default".to_string());
    let mmf_namespace = env::var("MMF_NAMESPACE").unwrap_or("default".to_string());
    let alloc_client = AgonesGameServerAllocationClient::new(gameserver_namespace)?;
    let director = OpenMatchDirector::new(alloc_client, om_backend_address, mmf_namespace).await?;
    let mut worker = Worker::new(director)?;
    worker.run().await;
    Ok(())
}