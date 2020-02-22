mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    service::run_server().await?;
    Ok(())
}
