use std::net::TcpStream;
use std::net::TcpListener;
use rocket::State;
use std::sync::RwLock;
use std::thread;

// Players have a role, a last vote, and a number
struct Player{
    role: Role,
    status: Status,
    id: u8
}

enum Status{
    Alive,
    Dead
}

enum Role{
    Innocent,
    Detective,
    Doctor,
    Wolf
}

struct ConnectedPlayer{
    stream: RwLock<TcpStatus>,
    player: Option<Player>
}

enum TcpStatus{
    Listening(TcpListener),
    Connected(TcpStream)
}

impl ConnectedPlayer{
    fn open_connections<'a>(&mut self){
        thread::spawn(move || {
            let mut stream = self.stream.write().unwrap();
            if let TcpStatus::Listening(listener) = &*stream{
                if let Result::Ok((out_stream, _addr)) = listener.accept(){
                    *stream = TcpStatus::Connected(out_stream)
                }
            }
        });
    }
}

#[get("/new_connection")]
pub fn new_connection(game_list: State<GameList>) -> String{
    let mut games = game_list.inner().games.write().unwrap();
    for game in games.iter_mut(){
        if game.players.len() < game.max_players as usize{

            let listener = TcpListener::bind("localhost:0").unwrap();
            let port = listener.local_addr().unwrap().port().to_string();

            let new_player = ConnectedPlayer{
                stream: RwLock::new(TcpStatus::Listening(listener)),
                player: None,
            };
            game.players.push(new_player);
            return port;
        }
    }
    let mut game = Game::new(8);
    let listener = TcpListener::bind("localhost:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();

    let new_player = ConnectedPlayer{
        stream: RwLock::new(TcpStatus::Listening(listener)),
        player: None,
    };

    game.players.push(new_player);
    games.push(game);
    return port;
}

struct Game{
    players: Vec<ConnectedPlayer>,
    phase: Phase,
    mafia_left: u8,
    innocent_left: u8,
    max_players: u8,
}

impl Game{
    pub fn new(max_players: u8) -> Game{
        Game{
            players: vec![],
            phase: Phase::Start,
            mafia_left: 0,
            innocent_left: 0,
            max_players: max_players,
        }
    }
}

enum Phase{
    Start,
    PreVote,
    Detect,
    Vote,
    Save,
    Kill,
}

pub struct GameList{
    games: RwLock<Vec<Game>>
}

impl GameList{
    pub fn new() -> GameList{
        GameList{
            games: RwLock::new(vec![])
        }
    }
}