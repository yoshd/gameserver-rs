use std::env;

use director_worker::{
    AgonesGameServerAllocationClient, AgonesSDKSelfAllocationClient, OpenMatchDirector, Worker,
};
use gameserver_client::GameServerClientImpl;

pub async fn run_worker() -> anyhow::Result<()> {
    let om_backend_address = env::var("OM_BACKEND_ADDRESS")
        .unwrap_or("om-backend.open-match.svc.cluster.local:50505".to_string());
    let gameserver_namespace = env::var("GAMESERVER_NAMESPACE").unwrap_or("default".to_string());
    let mmf_namespace = env::var("MMF_NAMESPACE").unwrap_or("default".to_string());
    let mode = env::var("GS_ALLOCATION_MODE").unwrap_or("outside".to_string()); // outside or self
    match mode.as_str() {
        "outside" => {
            let alloc_client = AgonesGameServerAllocationClient::new(gameserver_namespace)?;
            let director =
                OpenMatchDirector::new(alloc_client, om_backend_address, mmf_namespace).await?;
            let mut worker = Worker::new(director)?;
            worker.run().await;
        }
        "self" => {
            let sdk = agones::Sdk::new().expect("could not connect to the sidecar");
            let max_allocate = env::var("GS_MAX_ALLOCATE").unwrap_or("10".to_string());
            let gs_address = env::var("GS_ADDRESS").unwrap_or("localhost:10000".to_string());
            let gs_client = GameServerClientImpl::new(gs_address)
                .await
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            let alloc_client =
                AgonesSDKSelfAllocationClient::new(sdk, gs_client, max_allocate.parse()?);
            let director =
                OpenMatchDirector::new(alloc_client, om_backend_address, mmf_namespace).await?;
            let mut worker = Worker::new(director)?;
            worker.run().await;
        }
        _ => {
            panic!("invalid mode");
        }
    };
    Ok(())
}
