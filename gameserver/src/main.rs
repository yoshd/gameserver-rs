use std::env;
use std::time::Duration;

use log::{debug, error, info};
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

    // run server
    let address = env::var("ADDRESS").unwrap_or("0.0.0.0:10000".to_string());
    services::run_server(sdk.clone(), &address).await?;
    Ok(())
}
