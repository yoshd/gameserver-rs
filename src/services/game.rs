use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::mpsc;
use tonic::{transport::Server, Status};

pub mod pb {
    tonic::include_proto!("game");
}
use super::super::entities;

pub struct GameService {
    pub message_sender: mpsc::Sender<Result<pb::Message, tonic::Status>>,
    pub agones_sdk: agones::Sdk,
}

#[tonic::async_trait]
impl pb::game_server::Game for GameService {
    type JoinStream = mpsc::Receiver<Result<pb::Message, Status>>;
    async fn join(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Message>>,
    ) -> Result<tonic::Response<Self::JoinStream>, tonic::Status> {
        let (tx, rx) = mpsc::channel(1);

        let player_id = request
            .metadata()
            .get("player_id")
            .ok_or(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "please specify player_id",
            ))
            .and_then(|player_id| {
                player_id.to_str().map_err(|_| {
                    tonic::Status::new(tonic::Code::InvalidArgument, "please specify player_id")
                })
            })?;

        info!("joined player. player_id: {:?}", player_id);

        let player = entities::game_session::Player {
            id: player_id.to_string(),
            sender: tx.clone(),
        };

        {
            let mut g = GAME_SESSION
                .write()
                .map_err(|err| tonic::Status::new(tonic::Code::Aborted, err.to_string()))?;
            g.add_player(player.clone());
        }
        let mut s = self.message_sender.clone();
        let stream = request.into_inner();
        let sdk = self.agones_sdk.clone();
        tokio::spawn(async move {
            futures::pin_mut!(stream);
            let mut tx = tx.clone();
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(message) => {
                        match s.send(Ok(message.clone())).await {
                            Ok(_) => {}
                            Err(err) => {
                                error!("failed to send message: {:?}", err);
                                break;
                            }
                        };
                    }
                    Err(err) => {
                        error!("stream error: {:?}", err);
                        match tx
                            .send(Err(tonic::Status::new(
                                tonic::Code::Aborted,
                                err.to_string(),
                            )))
                            .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                error!("failed to send error message: {:?}", err);
                            }
                        };
                        break;
                    }
                }
            }
            {
                match GAME_SESSION.write() {
                    Ok(mut g) => {
                        g.delete_player(player.id);
                        if g.num_players() == 0 {
                            if sdk.shutdown().is_err() {
                                error!("Agones SDK shutdown failed");
                            }
                        }
                    }
                    Err(err) => error!("{:?}", err),
                }
            }
        });
        Ok(tonic::Response::new(rx))
    }
}

lazy_static! {
    pub static ref GAME_SESSION: Arc<RwLock<entities::game_session::GameSession<pb::Message, tonic::Status>>> =
        Arc::new(RwLock::new(entities::game_session::GameSession::new()));
}

// Todo: Consider a better way
// Currently, this is done because of the following problems:
// https://tokio-rs.github.io/tokio/doc/tokio/fn.spawn.html
pub fn clone_players() -> Vec<entities::game_session::Player<pb::Message, tonic::Status>> {
    let g = match GAME_SESSION.read() {
        Ok(g) => g,
        Err(err) => panic!("failed to get lock: {:?}", err), // todo: return Result<T, E>
    };
    g.players.clone()
}

pub async fn run_server(
    tx: tokio::sync::mpsc::Sender<Result<pb::Message, tonic::Status>>,
    sdk: agones::Sdk,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("start server");
    let addr = addr.parse().unwrap();
    let game_service = GameService {
        message_sender: tx,
        agones_sdk: sdk.clone(),
    };
    let svc = pb::game_server::GameServer::new(game_service);
    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await
        .map_err(|e| format!("could not start game server: {:?}", e))?;
    Ok(())
}

pub async fn run_worker(
    mut rx: tokio::sync::mpsc::Receiver<Result<pb::Message, tonic::Status>>,
    sdk: agones::Sdk,
) {
    info!("start worker");
    while let Some(msg) = rx.recv().await {
        match msg {
            Ok(message) => {
                let players = clone_players();
                for mut player in players {
                    match player.send_message(message.clone()).await {
                        Ok(_) => debug!("sent message"),
                        Err(err) => {
                            error!("failed to send message: {:?}", err);
                            {
                                match GAME_SESSION.write() {
                                    Ok(mut g) => {
                                        g.delete_player(player.id);
                                        if g.num_players() == 0 {
                                            if sdk.shutdown().is_err() {
                                                error!("agones sdk");
                                            }
                                        }
                                    }
                                    Err(err) => error!("{:?}", err),
                                }
                            }
                        }
                    };
                }
            }
            Err(err) => {
                error!("invalid message: {:?}", err);
                continue;
            }
        }
    }
}
