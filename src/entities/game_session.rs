use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Player<T, E> {
    pub id: String,
    pub sender: mpsc::Sender<Result<T, E>>,
}

impl<T, E> Player<T, E> {
    pub async fn send_message(
        &mut self,
        message: T,
    ) -> Result<(), mpsc::error::SendError<std::result::Result<T, E>>> {
        self.sender.send(Ok(message)).await
    }
}

pub struct GameSession<T, E> {
    pub players: Vec<Player<T, E>>,
}

impl<T, E> GameSession<T, E> {
    pub fn new() -> GameSession<T, E> {
        GameSession {
            players: Vec::new(),
        }
    }

    pub fn add_player(&mut self, player: Player<T, E>) {
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
