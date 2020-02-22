use std::env;

pub mod mm {
    tonic::include_proto!("matchmaker");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = env::var("SERVER_ADDR").unwrap();
    let address = format!("http://{}", address);
    for _ in 0..10 {
        let mut client = mm::frontend_client::FrontendClient::connect(address.clone()).await?;
        let mut stream = client
            .create_match(mm::CreateMatchRequest {
                player_id: "123".to_string(),
            })
            .await?
            .into_inner();
        while let Some(res) = stream.message().await? {
            println!("res={:?}", res);
        }
    }
    Ok(())
}
