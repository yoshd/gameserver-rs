use std::env;
use std::time::Duration;

use http::header::HeaderValue;
use tokio::time;
use tonic::transport::Channel;

pub mod game {
    tonic::include_proto!("game");
}

async fn run_message_stream(
    client: &mut game::game_client::GameClient<Channel>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = time::Instant::now();
    let outbound = async_stream::stream! {
        let mut interval = time::interval(Duration::from_secs(1));
        while let time = interval.tick().await {
            println!("send message");
            let elapsed = time.duration_since(start);
            let message = game::Message {
                body: "aaa".as_bytes().to_vec(),
            };
            yield message;
        };
    };

    let response = client.join(tonic::Request::new(outbound)).await?;
    let mut inbound = response.into_inner();

    while let Some(message) = inbound.message().await? {
        println!("message = {:?}", message);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = env::var("SERVER_ADDR").unwrap();
    let address = format!("http://{}", address);
    let channel = Channel::from_shared(address)
        .unwrap()
        .intercept_headers(|headers| {
            headers.insert("player_id", HeaderValue::from_static("12345"));
        })
        .connect()
        .await?;
    let mut client = game::game_client::GameClient::new(channel);
    run_message_stream(&mut client).await?;

    Ok(())
}
