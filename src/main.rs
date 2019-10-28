#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod game;
mod mafia;

use game::{ConnectedPlayer, ConnectionStatus, Game, Player};
use rand::seq::SliceRandom;

fn main() {
    game::launch::<MafiaPlayer, Mafia>();
}

pub struct Mafia {
    players: Vec<ConnectedPlayer<MafiaPlayer>>,
    phase: Phase,
    day: usize,
    mafia_left: usize,
    innocent_left: usize,
    max_players: usize,
}

impl Mafia {
    fn create_role_vec(&self) -> Vec<Role> {
        let mut roles = vec![];
        roles.push(Role::Doctor);
        roles.push(Role::Detective);
        for _ in 0..self.max_players / 4 {
            roles.push(Role::Mafia);
        }
        while roles.len() < self.max_players as usize {
            roles.push(Role::Innocent);
        }
        roles
    }
}

impl Game<MafiaPlayer> for Mafia {
    fn new(max_players: usize) -> Mafia {
        Mafia {
            players: vec![],
            phase: Phase::Start,
            day: 0,
            mafia_left: 0,
            innocent_left: 0,
            max_players: max_players,
        }
    }

    fn run_game(&mut self) {
        match self.phase {
            Phase::Start => {
                // Check if all players are connected
                for player in self.players.iter_mut() {
                    if let ConnectionStatus::Error | ConnectionStatus::NotConnected =
                        player.check_connections()
                    {
                        self.phase = Phase::End;
                        return;
                    }
                }

                // If all players are connected, create roles, and assign them to players
                let mut roles = self.create_role_vec();
                let mut rng = rand::thread_rng();
                roles.shuffle(&mut rng);

                for (i, mut player) in self.players.iter_mut().enumerate() {
                    player.player = Some(MafiaPlayer {
                        role: roles.pop().unwrap(),
                        status: Status::Alive,
                        id: i as u8,
                        guesses: vec![0; 8],
                    });

                    let state = match &player.player {
                        Some(actual_player) => actual_player.get_state(),
                        None => "\n".to_string(),
                    };
                    let _ = player.send_state(state);
                }
                self.phase = Phase::Detect;
            }

            Phase::Detect => {
                let mut state = self.get_state();
                let mut detective = None;
                // Find the detective and send him the state of the game
                for player in self.players.iter_mut() {
                    match &player.player {
                        Some(actual_player) => {
                            if let Role::Detective = actual_player.role {
                                state = format!("{}, {}", actual_player.get_state(), state);
                                let _ = player.send_state(state);
                                detective = Some(player);
                                break;
                            }
                        }
                        None => {}
                    };
                }
                // Get input from detective
                let mut buf: Vec<u8> = vec![0; 8];
                if let Some(player) = &mut detective {
                    let _ = player.read_input(&mut buf);
                }
                println!("{}", std::str::from_utf8(&buf).unwrap());
                self.phase = Phase::PreVote;
                panic!("DAB");
            }
            _ => println!("Phase not implemented"),
        }
    }

    fn over(&self) -> bool {
        self.phase == Phase::End
    }

    fn player_list(&self) -> &Vec<ConnectedPlayer<MafiaPlayer>> {
        &self.players
    }

    fn player_list_mut(&mut self) -> &mut Vec<ConnectedPlayer<MafiaPlayer>> {
        &mut self.players
    }

    fn max_players(&self) -> usize {
        self.max_players
    }

    fn max_players_mut(&mut self) -> &mut usize {
        &mut self.max_players
    }

    fn get_state(&self) -> String {
        let mut state = format!("{}", self.phase);
        state = format!("{}, {}", state, self.day);
        for player in self.players.iter() {
            match &player.player {
                Some(actual_player) => {
                    state = format!("{}, {}", state, actual_player.get_public_state());
                }
                None => {}
            };
        }
        state
    }
}

#[derive(Eq, PartialEq)]
enum Phase {
    Start,
    Detect,
    PreVote,
    Vote,
    Save,
    Kill,
    End,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Start => write!(f, "Start"),
            Phase::Detect => write!(f, "Detect"),
            Phase::PreVote => write!(f, "PreVote"),
            Phase::Vote => write!(f, "Vote"),
            Phase::Save => write!(f, "Save"),
            Phase::Kill => write!(f, "Kill"),
            Phase::End => write!(f, "End"),
        }
    }
}

#[derive(Clone)]
struct MafiaPlayer {
    role: Role,
    status: Status,
    id: u8,
    guesses: Vec<u8>,
}

#[derive(Clone)]
enum Status {
    Alive,
    Dead,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Alive => write!(f, "Alive"),
            Status::Dead => write!(f, "Dead"),
        }
    }
}

#[derive(Clone)]
enum Role {
    Innocent,
    Detective,
    Doctor,
    Mafia,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Innocent => write!(f, "Innocent"),
            Role::Detective => write!(f, "Detective"),
            Role::Doctor => write!(f, "Doctor"),
            Role::Mafia => write!(f, "Mafia"),
        }
    }
}

impl Player for MafiaPlayer {
    fn get_state(&self) -> String {
        format!(
            "{}, {}, {}",
            format!("Player: {}", self.id),
            format!("Role: {}", self.role),
            format!("Status: {}", self.status)
        )
    }
}

impl MafiaPlayer {
    fn get_public_state(&self) -> String {
        let mut state = format!("Status: {}", self.status);
        for guess in self.guesses.iter() {
            state = format!("{}, {}", state, guess);
        }
        state
    }
}
