use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use futures::StreamExt;
use lazy_static::lazy_static;
use log::{error, info};
use tokio::sync::mpsc;
use tonic::{transport::Server, Status};

pub mod pb {
    tonic::include_proto!("game");
}
use super::entities;
use super::entities::MatchId;

pub struct GameService {
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
        let match_id = request
            .metadata()
            .get("match_id")
            .ok_or(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "please specify match_id",
            ))
            .and_then(|match_id| {
                match_id.to_str().map_err(|_| {
                    tonic::Status::new(tonic::Code::InvalidArgument, "please specify match_id")
                })
            })?;

        info!(
            "joined player. player_id: {}, match_id: {}",
            player_id, match_id
        );

        let player = entities::Player {
            id: player_id.to_string(),
            sender: tx.clone(),
        };

        let mut wtx: mpsc::Sender<
            Result<entities::Event<pb::Message, tonic::Status>, tonic::Status>,
        >;
        {
            wtx = match WORKER_CHANNEL_MAP.write() {
                Ok(mut w) => match w.get(match_id) {
                    Some(wtx) => wtx.clone(),
                    None => {
                        let (tx, rx) = mpsc::channel(1);
                        let sdk = self.agones_sdk.clone();
                        let _match_id = match_id.to_string();
                        tokio::spawn(async move {
                            let status_manager = AgonesStatusManager { agones_sdk: sdk };
                            let game_session = entities::GameSession::new();
                            let mut worker =
                                Worker::new(_match_id, status_manager, game_session, rx);
                            if let Err(err) = worker.run().await {
                                error!("worker error: {:?}", err);
                            }
                        });
                        w.insert(match_id.to_string(), tx.clone());
                        tx
                    }
                },
                Err(err) => return Err(tonic::Status::new(tonic::Code::Aborted, err.to_string())),
            };
            let event = entities::Event {
                join: Some(entities::JoinEvent {
                    player: player.clone(),
                }),
                leave: None,
                message: None,
            };
            wtx.send(Ok(event))
                .await
                .map_err(|err| tonic::Status::new(tonic::Code::Aborted, err.to_string()))?;
        }

        let stream = request.into_inner();
        tokio::spawn(async move {
            futures::pin_mut!(stream);
            let mut tx = tx.clone();
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(message) => {
                        let event = entities::Event {
                            join: None,
                            leave: None,
                            message: Some(message.clone()),
                        };
                        if let Err(err) = wtx.send(Ok(event)).await {
                            error!("failed to send message: {:?}", err);
                            break;
                        }
                    }
                    Err(err) => {
                        error!("stream error: {:?}", err);
                        if let Err(err) = tx
                            .send(Err(tonic::Status::new(
                                tonic::Code::Aborted,
                                err.to_string(),
                            )))
                            .await
                        {
                            error!("failed to send error message: {:?}", err);
                        }
                        break;
                    }
                }
            }
        });
        Ok(tonic::Response::new(rx))
    }

    async fn get_server_info(
        &self,
        _request: tonic::Request<pb::GetServerInfoRequest>,
    ) -> Result<tonic::Response<pb::GetServerInfoResponse>, tonic::Status> {
        let mut num_matches = 0;
        {
            let w = WORKER_CHANNEL_MAP
                .read()
                .map_err(|err| tonic::Status::new(tonic::Code::Aborted, err.to_string()))?;
            num_matches = w.len() as i32;
        }
        let res = pb::GetServerInfoResponse {
            number_of_matches: num_matches,
        };
        Ok(tonic::Response::new(res))
    }
}

pub trait StatusManager {
    fn ready(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Worker<SM, M, E>
where
    SM: StatusManager,
{
    match_id: String,
    status_manager: SM,
    game_session: entities::GameSession<M, E>,
    rx: mpsc::Receiver<Result<entities::Event<M, E>, E>>,
}

impl<SM, M, E> Worker<SM, M, E>
where
    SM: StatusManager,
    M: Send + Clone + std::fmt::Debug,
    E: std::fmt::Debug,
{
    pub fn new(
        match_id: String,
        status_manager: SM,
        game_session: entities::GameSession<M, E>,
        rx: mpsc::Receiver<Result<entities::Event<M, E>, E>>,
    ) -> Worker<SM, M, E> {
        Worker {
            match_id: match_id,
            status_manager: status_manager,
            game_session: game_session,
            rx: rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("start worker");
        while let Some(event) = self.rx.recv().await {
            if let Ok(event) = event {
                if let Some(message) = event.message {
                    let mut failed_player = Vec::new();
                    for player in &mut self.game_session.players {
                        if let Err(err) = player.send_message(message.clone()).await {
                            error!("failed to send message: {:?}", err);
                            failed_player.push(player.id.clone());
                        }
                    }
                    for id in failed_player {
                        self.game_session.delete_player(id);
                        if self.game_session.num_players() == 0 {
                            {
                                match WORKER_CHANNEL_MAP.write() {
                                    Ok(mut w) => {
                                        w.remove(&self.match_id.clone());
                                        if w.len() == 0 {
                                            if self.status_manager.shutdown().is_err() {
                                                error!("failed to shutdown");
                                            }
                                        }
                                    }
                                    Err(err) => error!("{:?}", err),
                                };
                            }
                            return Ok(());
                        }
                    }
                    break;
                }
                if let Some(join) = event.join {
                    self.game_session.add_player(join.player);
                    break;
                }
                if let Some(leave) = event.leave {
                    self.game_session.delete_player(leave.player_id);
                    if self.game_session.num_players() == 0 {
                        {
                            match WORKER_CHANNEL_MAP.write() {
                                Ok(mut w) => {
                                    w.remove(&self.match_id.clone());
                                    if w.len() == 0 {
                                        if self.status_manager.shutdown().is_err() {
                                            error!("failed to shutdown");
                                        }
                                    }
                                }
                                Err(err) => error!("{:?}", err),
                            };
                        }
                        return Ok(());
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}

pub struct AgonesStatusManager {
    agones_sdk: agones::Sdk,
}

impl StatusManager for AgonesStatusManager {
    fn ready(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.agones_sdk
            .ready()
            .map_err(|err| format!("could not run ready(): {:?}", err))?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.agones_sdk
            .shutdown()
            .map_err(|err| format!("failed to shutdown: {:?}", err))?;
        Ok(())
    }
}

lazy_static! {
    pub static ref WORKER_CHANNEL_MAP: Arc<
        RwLock<
            HashMap<
                MatchId,
                mpsc::Sender<Result<entities::Event<pb::Message, tonic::Status>, tonic::Status>>,
            >,
        >,
    > = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn run_server(sdk: agones::Sdk, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("start server");
    let addr = addr.parse().unwrap();
    let game_service = GameService {
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
