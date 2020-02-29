use std::env;

use director_worker::{AgonesGameServerAllocationClient, OpenMatchDirector, Worker};

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
