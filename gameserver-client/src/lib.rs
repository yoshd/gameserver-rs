use async_trait::async_trait;
use tonic::transport::Channel;

mod game {
    tonic::include_proto!("game");
}

#[async_trait]
pub trait GameServerClient {
    async fn get_number_of_matches(&self) -> Result<i32, Box<dyn std::error::Error>>;
}

pub struct GameServerClientImpl {
    client: game::game_client::GameClient<Channel>,
}

impl GameServerClientImpl {
    pub async fn new(address: String) -> Result<Self, Box<dyn std::error::Error>> {
        let address = format!("http://{}", address);
        let client = game::game_client::GameClient::connect(address).await?;
        Ok(GameServerClientImpl { client: client })
    }
}

#[async_trait]
impl GameServerClient for GameServerClientImpl {
    async fn get_number_of_matches(&self) -> Result<i32, Box<dyn std::error::Error>> {
        let mut client = self.client.clone();
        let req = game::GetServerInfoRequest {};
        let res = client.get_server_info(tonic::Request::new(req)).await?;
        Ok(res.into_inner().number_of_matches)
    }
}
