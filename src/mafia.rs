use rocket::State;
use std::net::TcpListener;
use std::net::TcpStream;
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
    Wolf,
}

struct ConnectedPlayer {
    stream: TcpStatus,
    player: Option<Player>,
}

enum TcpStatus {
    Uninitialized(TcpListener),
    Listening(JoinHandle<Result<TcpStream, TcpListener>>),
    Connected(TcpStream),
}

// NETWORKING LOGIC

impl ConnectedPlayer {
    fn open_connections(mut self) -> Self {
        if let TcpStatus::Uninitialized(listener) = self.stream {
            self.stream = TcpStatus::Listening(thread::spawn(|| ConnectedPlayer::listen(listener)));
        }
        self
    }

    fn listen(stream: TcpListener) -> Result<TcpStream, TcpListener> {
        if let Result::Ok((out_stream, _addr)) = stream.accept() {
            return Result::Ok(out_stream);
        }
        Result::Err(stream)
    }
    fn check_connections(&mut self) {
        if let TcpStatus::Listening(handle) = &self.stream {
            match handle.join().unwrap() {
                Ok(stream) => {
                    self.stream = TcpStatus::Connected(stream);
                }
                Err(listener) => self.stream = TcpStatus::Uninitialized(listener),
            }
        }
    }
}

#[get("/new_connection")]
pub fn new_connection(game_list: State<Arc<GameList>>) -> String {
    let mut games = game_list.inner().games.lock().unwrap();
    for game in games.iter_mut() {
        if game.players.len() < game.max_players as usize {
            let listener = TcpListener::bind("localhost:0").unwrap();
            let port = listener.local_addr().unwrap().port().to_string();

            let new_player = ConnectedPlayer {
                stream: TcpStatus::Uninitialized(listener),
                player: None,
            }
            .open_connections();
            game.players.push(new_player);
            return port;
        }
    }
    let mut game = Game::new(8);
    let listener = TcpListener::bind("localhost:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();

    let new_player = ConnectedPlayer {
        stream: TcpStatus::Uninitialized(listener),
        player: None,
    }
    .open_connections();

    game.players.push(new_player);
    games.push(game);
    return port;
}

pub struct GameList {
    pub games: Mutex<Vec<Game>>,
}

impl GameList {
    pub fn new() -> GameList {
        GameList {
            games: Mutex::new(vec![]),
        }
    }
}

pub fn check_games(in_list: Arc<GameList>, out: Sender<Game>) -> JoinHandle<()> {
    thread::spawn(move || loop {
        let mut games = in_list.games.lock().unwrap();
        let mut remove_games: Vec<usize> = vec![];
        for (i, game) in games.iter().enumerate() {
            if game.players.len() == game.max_players as usize {
                remove_games.push(i);
            }
        }
        for num in remove_games.drain(..).rev() {
            let _ = out.send(games.remove(num)).unwrap();
        }
    })
}

pub fn run_active_games(active_games: GameList, input: Receiver<Game>) -> JoinHandle<()> {
    thread::spawn(move || loop {
        let mut games = active_games.games.lock().unwrap();
        for game in input.try_iter() {
            games.push(game);
        }
        for game in games.iter_mut() {
            game.run_game();
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

    fn run_game(&mut self) {
        match self.phase {
            Phase::Start => {
                for player in self.players.iter_mut() {
                    player.check_connections();
                }
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
}
