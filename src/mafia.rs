use rand::seq::SliceRandom;
use rocket::State;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;

// Players have a role, a last vote, and a number
struct Player {
    role: Role,
    status: Status,
    id: u8,
}

enum Status {
    Alive,
    Dead,
}

enum Role {
    Innocent,
    Detective,
    Doctor,
    Mafia,
}

pub struct ConnectedPlayer {
    stream: TcpStatus,
    player: Option<Player>,
}

enum TcpStatus {
    Uninitialized(TcpListener),
    Listening(JoinHandle<Result<TcpStream, TcpListener>>, Receiver<bool>),
    Connected(TcpStream),
    Hold,
    ConnectionError,
}

// NETWORKING LOGIC

impl ConnectedPlayer {
    fn open_connections(mut self) -> Self {
        let (send, recieve) = channel::<bool>();
        if let TcpStatus::Uninitialized(listener) = self.stream {
            self.stream = TcpStatus::Listening(
                thread::spawn(|| ConnectedPlayer::listen(listener, send)),
                recieve,
            );
        }
        self
    }

    fn listen(stream: TcpListener, send: Sender<bool>) -> Result<TcpStream, TcpListener> {
        if let Result::Ok((out_stream, _addr)) = stream.accept() {
            let _ = send.send(true);
            return Result::Ok(out_stream);
        }
        Result::Err(stream)
    }

    fn check_connections(&mut self) -> bool {
        let mut hold = TcpStatus::Hold;

        use std::mem;
        mem::swap(&mut self.stream, &mut hold);

        // Temp solution
        if let TcpStatus::Listening(handle, recieve) = hold {
            if recieve.try_iter().count() != 0 {
                match handle.join().unwrap() {
                    Ok(stream) => {
                        self.stream = TcpStatus::Connected(stream);
                        return true;
                    }
                    Err(_) => {
                        self.stream = TcpStatus::ConnectionError;
                        return false;
                    }
                }
            } else {
                self.stream = TcpStatus::Listening(handle, recieve);
            }
        }
        return false;
    }
}

#[get("/new_connection")]
pub fn new_connection(player_channel: State<Mutex<Sender<ConnectedPlayer>>>) -> String {
    let listener = TcpListener::bind("localhost:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();

    let new_player = ConnectedPlayer {
        stream: TcpStatus::Uninitialized(listener),
        player: None,
    }
    .open_connections();

    let _ = player_channel.lock().unwrap().send(new_player);
    return port;
}

pub fn check_games(out: Sender<Game>, players: Receiver<ConnectedPlayer>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut game: Option<Game> = None;
        loop {
            for player in players.try_iter() {
                match game {
                    Some(mut lobby) => {
                        lobby.players.push(player);
                        if lobby.players.len() > lobby.max_players as usize {
                            let _ = out.send(lobby);
                            game = None;
                            println!("Sent game over!");
                        } else {
                            game = Some(lobby);
                        }
                    }
                    None => {
                        let mut lobby = Game::new(8);
                        lobby.players.push(player);
                        game = Some(lobby);
                    }
                }
            }
        }
    })
}

pub fn run_active_games(
    input: Receiver<Game>,
    out_players: Sender<ConnectedPlayer>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut games: Vec<Game> = Vec::new();
        loop {
            for game in input.try_iter() {
                println!("Got game!");
                games.push(game);
            }
            let mut good_games = vec![];
            for mut game in games.drain(..) {
                game.run_game();
                if let Phase::End = game.phase {
                    for player in game.players.drain(..) {
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

// /NETWORKING LOGIC

pub struct Game {
    players: Vec<ConnectedPlayer>,
    phase: Phase,
    mafia_left: u8,
    innocent_left: u8,
    max_players: u8,
}

impl Game {
    fn new(max_players: u8) -> Game {
        Game {
            players: vec![],
            phase: Phase::Start,
            mafia_left: 0,
            innocent_left: 0,
            max_players: max_players,
        }
    }

    fn create_role_vec(&self) -> Vec<Role> {
        let mut roles = vec![];
        roles.push(Role::Doctor);
        roles.push(Role::Detective);
        for i in 0..self.max_players / 4 {
            roles.push(Role::Mafia);
        }
        while roles.len() < self.max_players as usize {
            roles.push(Role::Innocent);
        }
        roles
    }

    fn run_game(&mut self) {
        match self.phase {
            Phase::Start => {
                // Check if all players are connected
                let mut all_players_ready = true;
                for player in self.players.iter_mut() {
                    if !player.check_connections() {
                        all_players_ready = false;
                    }
                }

                // If all players are connected, create roles, and assign them to players
                let mut roles = self.create_role_vec();
                let mut rng = rand::thread_rng();
                roles.shuffle(&mut rng);

                if all_players_ready {
                    for (i, player) in self.players.iter_mut().enumerate() {
                        player.player = Some(Player {
                            role: roles.pop().unwrap(),
                            status: Status::Alive,
                            id: i as u8,
                        });
                    }
                }
                self.phase = Phase::Detect;
            }
            _ => println!("Phase not implemented"),
        }
    }
}

enum Phase {
    Start,
    Detect,
    PreVote,
    Vote,
    Save,
    Kill,
    End,
}
