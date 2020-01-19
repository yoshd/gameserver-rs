use log::{debug, error, info};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

mod entities;
mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("start gameserver");
    let sdk = agones::Sdk::new().map_err(|_| "could not connect to the sidecar")?;

    // health check
    let mut _sdk = sdk.clone();
    tokio::spawn(async move {
        let mut sdk = _sdk.clone();
        info!("start health check");
        let mut interval = time::interval(Duration::from_millis(2000));
        loop {
            match sdk.health() {
                (s, Ok(_)) => {
                    debug!("health check is OK");
                    sdk = s;
                }
                (s, Err(e)) => {
                    error!("health check error: {:?}", e);
                    sdk = s;
                }
            }
            interval.tick().await;
        }
    });
    // marking server as ready
    sdk.ready()
        .map_err(|e| format!("could not run ready(): {:?}", e))?;

    let (tx, rx) = mpsc::channel(1);
    // run message worker
    let _sdk = sdk.clone();
    tokio::spawn(async move {
        services::game::run_worker(rx, _sdk).await;
    });
    // run server
    services::game::run_server(tx, sdk.clone(), "0.0.0.0:10000").await?;
    Ok(())
}
