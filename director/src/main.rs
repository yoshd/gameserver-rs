mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    worker::run_worker().await
}
