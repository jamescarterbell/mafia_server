pub mod connected_player;

pub use connected_player::*;
use rocket::http::Method;
use rocket::*;
use std::io::Cursor;
use std::marker::Send;
use std::sync::mpsc::*;
use std::thread::JoinHandle;

pub fn new_connection<P>(player_channel: &std::sync::mpsc::Sender<ConnectedPlayer<P>>) -> String
where
    P: Player + Send + 'static,
{
    let mut port = "00000".to_string();

    let mut new_player = ConnectedPlayer::new();
    if let SocketStatus::Uninitialized(socket) = new_player.socket {
        port = socket.local_addr().unwrap().port().to_string();
        new_player.socket = SocketStatus::Uninitialized(socket);
    }
    new_player = new_player.open_connections();

    let _ = player_channel.send(new_player);
    return port;
}

#[derive(Clone)]
pub struct Connector<P>(std::sync::mpsc::Sender<ConnectedPlayer<P>>)
where
    P: Send + Player + 'static;

impl<P> rocket::Handler for Connector<P>
where
    P: Send + Player + Clone + 'static,
{
    fn handle<'r>(
        &self,
        _req: &'r rocket::Request,
        _data: Data,
    ) -> rocket::Outcome<rocket::Response<'r>, rocket::http::Status, rocket::Data> {
        rocket::Outcome::Success({
            let mut res = rocket::Response::new();
            res.set_sized_body(Cursor::new(new_connection(&self.0)));
            res
        })
    }
}

unsafe impl<P> std::marker::Sync for Connector<P> where P: Send + Player {}

pub fn launch<P, G>()
where
    P: Player + Send + Clone + 'static,
    G: Game<P> + Send + 'static,
{
    let (send_games, recieve_games) = channel::<G>();
    let (send_players, recieve_players) = channel::<ConnectedPlayer<P>>();

    check_games::<P, G>(send_games, recieve_players);
    run_active_games::<P, G>(recieve_games, send_players.clone());
    let route = Route::new(Method::Get, "/new_connection", Connector(send_players));
    rocket::ignite().mount("/", vec![route]).launch();
}

fn check_games<P, G>(
    out: std::sync::mpsc::Sender<G>,
    players: Receiver<ConnectedPlayer<P>>,
) -> JoinHandle<()>
where
    P: Player + Send + 'static,
    G: Game<P> + Send + 'static,
{
    std::thread::spawn(move || {
        let mut game: Option<G> = None;
        loop {
            for player in players.try_iter() {
                match game {
                    Some(mut lobby) => {
                        let players = lobby.player_list_mut();
                        players.push(player);
                        if players.len() >= lobby.max_players() as usize {
                            let _ = out.send(lobby);
                            game = None;
                        } else {
                            game = Some(lobby);
                        }
                    }
                    None => {
                        let mut lobby = G::new(8);
                        lobby.player_list_mut().push(player);
                        game = Some(lobby);
                    }
                }
            }
        }
    })
}

pub fn run_active_games<P, G>(
    input: Receiver<G>,
    out_players: std::sync::mpsc::Sender<ConnectedPlayer<P>>,
) -> JoinHandle<()>
where
    P: Player + Send + 'static,
    G: Game<P> + Send + 'static,
{
    std::thread::spawn(move || {
        let mut games: Vec<G> = Vec::new();
        loop {
            for game in input.try_iter() {
                games.push(game);
            }
            let mut good_games = vec![];
            for mut game in games.drain(..) {
                game.run_game();
                if game.error() {
                    for mut player in game.player_list_mut().drain(..) {
                        if let ConnectionStatus::Error = player.check_connections() {
                            continue;
                        }
                        let _ = out_players.send(player);
                    }
                } else if game.over() {
                    for mut player in game.player_list_mut().drain(..) {
                        if let ConnectionStatus::Error = player.check_connections() {
                            continue;
                        }
                        let _ = out_players.send(player);
                    }
                } else {
                    good_games.push(game);
                }
            }
            games = good_games;
        }
    })
}

pub trait Game<P>
where
    P: connected_player::Player + std::marker::Send + 'static,
{
    fn new(players: usize) -> Self;
    fn run_game(&mut self);
    fn error(&self) -> bool;
    fn over(&self) -> bool;
    fn player_list(&self) -> &Vec<ConnectedPlayer<P>>;
    fn player_list_mut(&mut self) -> &mut Vec<ConnectedPlayer<P>>;
    fn max_players(&self) -> usize;
    fn max_players_mut(&mut self) -> &mut usize;
    fn get_state(&self) -> String;
}
