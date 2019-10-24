pub mod connected_player;
pub mod game_trait;

use connected_player::*;
use rocket::*;
use std::marker::Send;
use std::sync::mpsc;
use std::sync::Mutex;
use ws::*;

#[get("/new_connection")]
pub fn new_connection<T>(player_channel: State<Mutex<mpsc::Sender<ConnectedPlayer<T>>>>) -> String
where
    T: Player + Send,
{
    let mut port: String;

    let new_player = ConnectedPlayer::new();
    if let SocketStatus::Uninitialized(socket) = new_player.socket {
        new_player.socket = SocketStatus::Uninitialized(socket.bind("127.0.0.1:00000").unwrap());
        port = socket.local_addr().unwrap().port().to_string();
    }

    let _ = player_channel.lock().unwrap().send(new_player);
    return port;
}
