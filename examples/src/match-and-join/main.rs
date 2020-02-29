use std::env;
use std::time::Duration;

use tokio::time;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use uuid::Uuid;

pub mod game {
    tonic::include_proto!("game");
}

pub mod mm {
    tonic::include_proto!("matchmaker");
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
    let mm_address = format!("http://{}", env::var("MM_SERVER_ADDR").unwrap());
    for _ in 0..10 {
        let mm_address = mm_address.clone();
        tokio::spawn(async move {
            let mut mm_client = mm::frontend_client::FrontendClient::connect(mm_address)
                .await
                .unwrap();
            let player_id = Uuid::new_v4().to_string();
            let mut stream = mm_client
                .create_match(mm::CreateMatchRequest {
                    player_id: player_id.clone(),
                })
                .await
                .unwrap()
                .into_inner();
            let mut gs_address = "".to_string();
            while let Some(res) = stream.message().await.unwrap() {
                println!(
                    "successful matchmakin! player_id:{}, gameserver: {:?}",
                    player_id, res.game_server
                );
                gs_address = format!("http://{}", res.game_server.unwrap().address);
                break;
            }
            let channel = Channel::from_shared(gs_address)
                .unwrap()
                .connect()
                .await
                .unwrap();
            let metadata = MetadataValue::from_str(&player_id).unwrap();
            let mut client = game::game_client::GameClient::with_interceptor(
                channel,
                move |mut req: tonic::Request<()>| {
                    req.metadata_mut().insert("player_id", metadata.clone());
                    Ok(req)
                },
            );
            run_message_stream(&mut client).await.unwrap();
        });
    }
    // 雑
    tokio::time::delay_for(std::time::Duration::from_secs(30)).await;
    Ok(())
}
