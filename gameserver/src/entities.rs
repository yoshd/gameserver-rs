use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct Player<M, E> {
    pub id: String,
    pub sender: mpsc::Sender<Result<M, E>>,
}

impl<M, E> Player<M, E> {
    pub async fn send_message(
        &mut self,
        message: M,
    ) -> Result<(), mpsc::error::SendError<std::result::Result<M, E>>> {
        self.sender.send(Ok(message)).await
    }
}

pub type MatchId = String;

pub struct GameSession<M, E> {
    pub players: Vec<Player<M, E>>,
}

impl<M, E> GameSession<M, E> {
    pub fn new() -> GameSession<M, E> {
        GameSession {
            players: Vec::new(),
        }
    }

    pub fn add_player(&mut self, player: Player<M, E>) {
        self.players.push(player);
    }

    pub fn delete_player(&mut self, id: String) {
        let mut index: i32 = -1;
        for (i, player) in self.players.iter().enumerate() {
            if player.id == id {
                index = i as i32;
            }
        }
        if index != -1 {
            self.players.swap_remove(index as usize);
        }
    }

    pub fn num_players(&self) -> usize {
        self.players.len()
    }
}

#[derive(Clone, Debug)]
pub struct JoinEvent<M, E> {
    pub player: Player<M, E>,
}

#[derive(Clone, Debug)]
pub struct LeaveEvent {
    pub player_id: String,
}

#[derive(Clone, Debug)]
pub struct Event<M, E> {
    pub join: Option<JoinEvent<M, E>>,
    pub leave: Option<LeaveEvent>,
    pub message: Option<M>,
}
