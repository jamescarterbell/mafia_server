#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod game;

use game::{ConnectedPlayer, ConnectionStatus, Game, Player, SocketStatus};
use rand::seq::SliceRandom;
use std::net::Shutdown;

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
        roles.push(Role::Detective);
        for _ in 0..self.max_players / 4 {
            roles.push(Role::Mafia);
        }
        while roles.len() < self.max_players as usize {
            roles.push(Role::Innocent);
        }
        roles
    }

    fn finished_game(&mut self) {
        // Create message containing hidden info
        let mut state = "End".to_string();
        for player in self.players.iter_mut() {
            if let Some(actual_player) = &player.player {
                state = format!("{}, {}", state, actual_player.get_state());
            };
        }

        // Tell all players hidden info
        println!("{}", state);
        let players = &mut self.players;
        for player in players.iter_mut() {
            if let Err(_) = player.send_state(state.clone()) {
                println!("Error ending game, closing game!");
                player.socket = SocketStatus::ConnectionError;
            };
        }

        // See if players want to quit the game
        for player in players.iter_mut() {
            let mut buf: Vec<u8> = vec![0; self.max_players];
            if let Err(_) = player.read_input(&mut buf) {
                println!("Error in End");
                player.socket = SocketStatus::ConnectionError;
                continue;
            };
            let test = std::str::from_utf8(&buf).unwrap().to_string();
            let out = read_input(test);
            if out.len() == 0 || out[0] == 0.0 {
                player.socket = SocketStatus::ConnectionError;
            }
        }
    }

    fn get_private_state(&self) -> String {
        let mut state = format!("Phase: {}", self.phase);
        state = format!("{}, Day: {}", state, self.day);
        for player in self.players.iter() {
            match &player.player {
                Some(actual_player) => {
                    state = format!("{}, {}", state, actual_player.get_private_state());
                }
                None => {}
            };
        }
        state
    }
}

impl Game<MafiaPlayer> for Mafia {
    fn new(max_players: usize) -> Mafia {
        Mafia {
            players: vec![],
            phase: Phase::Start,
            day: 0,
            mafia_left: max_players / 4,
            innocent_left: max_players - max_players / 4,
            max_players: max_players,
        }
    }

    fn run_game(&mut self) {
        let max_players = self.max_players();
        match self.phase {
            Phase::Start => {
                // Check if all players are connected
                for player in self.players.iter_mut() {
                    if let ConnectionStatus::Error = player.check_connections() {
                        self.phase = Phase::Error;
                        return;
                    }

                    if let ConnectionStatus::NotConnected = player.check_connections() {
                        return;
                    }
                }

                // If all players are connected, create roles, and assign them to players
                let mut roles = self.create_role_vec();
                let mut rng = rand::thread_rng();
                roles.shuffle(&mut rng);

                let mut mafia_members = Vec::<usize>::new();
                let mut mafia_states = Vec::<String>::new();

                for (i, mut player) in self.players.iter_mut().enumerate() {
                    player.player = Some(MafiaPlayer {
                        role: roles.pop().unwrap(),
                        status: Status::Alive,
                        id: i as u8,
                        guesses: vec![0.0; max_players],
                        hid_guess: vec![0.0; max_players],
                    });

                    let state = match &player.player {
                        Some(actual_player) => actual_player.get_state(),
                        None => "\n".to_string(),
                    };

                    if let Role::Mafia = player.player.as_ref().unwrap().role {
                        mafia_members.push(i);
                        mafia_states.push(state.clone());
                    }
                    if let Err(_) =
                        player.send_state(format!("{}, {}, {}", "Start", state, max_players))
                    {
                        player.socket = SocketStatus::ConnectionError;
                        println!("Error in Start, closing game!");
                        self.phase = Phase::End;
                        return;
                    };
                }

                for i in mafia_members.iter() {
                    let player = &mut self.players[*i];
                    for message in mafia_states.iter() {
                        if let Err(_) = player.send_state(message.clone()) {
                            player.socket = SocketStatus::ConnectionError;
                            println!("Error in Start, closing game!");
                            self.phase = Phase::End;
                            return;
                        }
                    }
                }

                self.phase = Phase::Detect;
            }

            Phase::Detect => {
                let mut state = self.get_state();
                let mut detective = None;
                let players = &mut self.players;
                // Find the detective and send him the state of the game
                for (i, player) in players.iter_mut().enumerate() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    match &player.player {
                        Some(actual_player) => {
                            if let Role::Detective = actual_player.role {
                                state = format!("Action, {}, {}", actual_player.get_state(), state);
                                if let Err(_) = player.send_state(state) {
                                    player.socket = SocketStatus::ConnectionError;
                                    println!("Error in Detect, closing game!");
                                    self.phase = Phase::End;
                                    return;
                                };
                                detective = Some(i);
                                break;
                            }
                        }
                        None => {}
                    };
                }

                if let None = &detective {
                    self.phase = Phase::PreVote;
                    return;
                }

                // Get input from detective
                let mut buf: Vec<u8> = vec![0; max_players];
                if let Some(player) = &detective {
                    if let Err(_) = &players.get_mut(*player).unwrap().read_input(&mut buf) {
                        players.get_mut(*player).unwrap().socket = SocketStatus::ConnectionError;
                        println!("Error in Detect, closing game!");
                        self.phase = Phase::End;
                        return;
                    };
                }

                // Get max of returned vec
                let out = read_input(std::str::from_utf8(&buf).unwrap().to_string());
                let max = max_index(&out);
                if let Some(max) = max {
                    // Get role of vote and send back to detective
                    state = players
                        .get_mut(max)
                        .unwrap()
                        .player
                        .as_ref()
                        .unwrap()
                        .get_state();
                    state = format!("Info, {}", state);
                    if let Some(player) = &mut detective {
                        if let Err(_) = &players.get_mut(*player).unwrap().send_state(state) {
                            players.get_mut(*player).unwrap().socket =
                                SocketStatus::ConnectionError;
                            println!("Error in Detect, closing game!");
                            self.phase = Phase::End;
                            return;
                        };
                    };
                }

                self.phase = Phase::PreVote;
            }

            Phase::PreVote => {
                let state = self.get_state();
                let players = &mut self.players;

                // Send all the players the state of the game
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    match &player.player {
                        Some(actual_player) => {
                            let local_state =
                                format!("Action, {}, {}", actual_player.get_state(), state);
                            if let Err(_) = player.send_state(local_state) {
                                println!("Error in Prevote, closing game!");
                                player.socket = SocketStatus::ConnectionError;
                                self.phase = Phase::End;
                                return;
                            };
                        }
                        None => {}
                    };
                }

                // Recieve all the players input
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    let mut buf: Vec<u8> = vec![0; max_players];
                    if let Err(_) = player.read_input(&mut buf) {
                        println!("Error in PreVote, closing game!");
                        player.socket = SocketStatus::ConnectionError;
                        self.phase = Phase::End;
                        return;
                    };
                    let out = read_input(std::str::from_utf8(&buf).unwrap().to_string());
                    match &mut player.player {
                        Some(actual_player) => {
                            actual_player.guesses = out.clone();
                            actual_player.hid_guess = out;
                        }
                        None => {}
                    };
                }
                self.phase = Phase::Vote;
            }

            Phase::Vote => {
                let state = self.get_state();
                let players = &mut self.players;

                // Send all the players the newest state
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    match &player.player {
                        Some(actual_player) => {
                            let local_state =
                                format!("Action, {}, {}", actual_player.get_state(), state);
                            if let Err(_) = player.send_state(local_state) {
                                player.socket = SocketStatus::ConnectionError;
                                println!("Error in Vote, closing game!");
                                self.phase = Phase::End;
                                return;
                            };
                        }
                        None => {}
                    };
                }

                // Collected the votes
                let mut votes: Vec<usize> = vec![0; max_players];
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    let mut buf: Vec<u8> = vec![0; max_players];
                    if let Err(_) = player.read_input(&mut buf) {
                        player.socket = SocketStatus::ConnectionError;
                        println!("Error in Vote, closing game!");
                        self.phase = Phase::End;
                        return;
                    };
                    let out = read_input(std::str::from_utf8(&buf).unwrap().to_string());
                    let max = max_index(&out);
                    if let Some(max) = max {
                        if out[max] <= 0.0 {
                            continue;
                        }
                        votes[max] += 1;
                        match &mut player.player {
                            Some(actual_player) => {
                                actual_player.guesses = out.clone();
                                actual_player.hid_guess = out;
                            }
                            None => {}
                        };
                    }
                }

                // Kill the target of the votes if majority
                let mut state = None;
                let verdict = max_index(&votes);
                if let Some(verdict) = verdict {
                    if votes[verdict] > (self.mafia_left + self.innocent_left) / 2 {
                        if let Some(player) = &mut players[verdict].player {
                            if let Status::Alive = player.status {
                                player.status = Status::Dead;
                                state = Some(player.get_state());
                                if let Role::Mafia = player.role {
                                    self.mafia_left -= 1;
                                } else {
                                    self.innocent_left -= 1;
                                }
                            }
                        }
                    }
                }
                // Send the info of the killed player out
                if let Some(state) = state {
                    for player in players.iter_mut() {
                        if let Status::Dead = player.get_status() {
                            continue;
                        }
                        match &player.player {
                            Some(actual_player) => {
                                if let Err(_) =
                                    player.send_state(format!("Info, {}", state.clone()))
                                {
                                    player.socket = SocketStatus::ConnectionError;
                                    println!("Error in Vote, closing game!");
                                    self.phase = Phase::End;
                                    return;
                                };
                            }
                            None => {}
                        };
                    }
                }

                if self.mafia_left == 0 || self.mafia_left >= self.innocent_left {
                    self.finished_game();
                    self.phase = Phase::End;
                    return;
                }
                self.phase = Phase::PreKill;
            }

            Phase::PreKill => {
                let state = self.get_private_state();
                let players = &mut self.players;

                // Send all the mafia players the state of the game
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }

                    if let Role::Detective | Role::Innocent = player.get_role() {
                        continue;
                    }

                    match &player.player {
                        Some(actual_player) => {
                            let local_state =
                                format!("Action, {}, {}", actual_player.get_state(), state);
                            if let Err(_) = player.send_state(local_state) {
                                player.socket = SocketStatus::ConnectionError;
                                println!("Error in PreKill, closing game!");
                                self.phase = Phase::End;
                                return;
                            };
                        }
                        None => {}
                    };
                }

                // Recieve all the mafia players input
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    if let Role::Detective | Role::Innocent = player.get_role() {
                        continue;
                    }
                    let mut buf: Vec<u8> = vec![0; max_players];
                    if let Err(_) = player.read_input(&mut buf) {
                        player.socket = SocketStatus::ConnectionError;
                        println!("Error in PreKill, closing game!");
                        self.phase = Phase::End;
                        return;
                    };
                    let out = read_input(std::str::from_utf8(&buf).unwrap().to_string());
                    match &mut player.player {
                        Some(actual_player) => {
                            actual_player.hid_guess = out;
                        }
                        None => {}
                    };
                }
                self.phase = Phase::Kill;
            }

            Phase::Kill => {
                let state = self.get_private_state();
                let players = &mut self.players;

                // Send all the mafia players the newest state
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    if let Role::Detective | Role::Innocent = player.get_role() {
                        continue;
                    }
                    match &player.player {
                        Some(actual_player) => {
                            let local_state =
                                format!("Action, {}, {}", actual_player.get_state(), state);
                            if let Err(_) = player.send_state(local_state) {
                                player.socket = SocketStatus::ConnectionError;
                                println!("Error in Kill, closing game!");
                                self.phase = Phase::End;
                                return;
                            };
                        }
                        None => {}
                    };
                }

                // Collected the votes
                let mut votes: Vec<usize> = vec![0; max_players];
                for player in players.iter_mut() {
                    if let Status::Dead = player.get_status() {
                        continue;
                    }
                    if let Role::Detective | Role::Innocent = player.get_role() {
                        continue;
                    }
                    let mut buf: Vec<u8> = vec![0; max_players];
                    if let Err(_) = player.read_input(&mut buf) {
                        player.socket = SocketStatus::ConnectionError;
                        println!("Error in Kill, closing game!");
                        self.phase = Phase::End;
                        return;
                    };
                    let out = read_input(std::str::from_utf8(&buf).unwrap().to_string());
                    let max = max_index(&out);
                    if let Some(max) = max {
                        votes[max] += 1;
                        match &mut player.player {
                            Some(actual_player) => {
                                actual_player.hid_guess = out;
                            }
                            None => {}
                        };
                    }
                }

                // Kill the target of the votes if majority
                let mut state = None;
                let verdict = max_index(&votes);
                if let Some(verdict) = verdict {
                    if votes[verdict] >= self.mafia_left / 2 {
                        if let Some(player) = &mut players[verdict].player {
                            if let Status::Alive = player.status {
                                player.status = Status::Dead;
                                state = Some(player.get_state());
                                if let Role::Mafia = player.role {
                                    self.mafia_left -= 1;
                                } else {
                                    self.innocent_left -= 1;
                                }
                            }
                        }
                    }
                }

                // Send the info of the killed player out
                if let Some(state) = state {
                    for player in players.iter_mut() {
                        if let Status::Dead = player.get_status() {
                            continue;
                        }
                        match &player.player {
                            Some(actual_player) => {
                                if let Err(_) =
                                    player.send_state(format!("Info, {}", state.clone()))
                                {
                                    player.socket = SocketStatus::ConnectionError;
                                    println!("Error in Vote, closing game!");
                                    self.phase = Phase::End;
                                    return;
                                };
                            }
                            None => {}
                        };
                    }
                }

                if self.mafia_left == 0 || self.mafia_left >= self.innocent_left || self.day >= 100
                {
                    self.finished_game();
                    self.phase = Phase::End;
                    return;
                }
                self.day += 1;
                self.phase = Phase::Detect;
            }
            _ => println!("Phase not implemented"),
        }
    }

    fn error(&self) -> bool {
        self.phase == Phase::Error
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
        let mut state = format!("Phase: {}", self.phase);
        state = format!("{}, Day: {}", state, self.day);
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
    PreKill,
    Kill,
    End,
    Error,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Start => write!(f, "Start"),
            Phase::Detect => write!(f, "Detect"),
            Phase::PreVote => write!(f, "PreVote"),
            Phase::Vote => write!(f, "Vote"),
            Phase::PreKill => write!(f, "PreKill"),
            Phase::Kill => write!(f, "Kill"),
            Phase::End => write!(f, "End"),
            Phase::Error => write!(f, "Error"),
        }
    }
}

#[derive(Clone)]
struct MafiaPlayer {
    role: Role,
    status: Status,
    id: u8,
    guesses: Vec<f64>,
    hid_guess: Vec<f64>,
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
enum Role {
    Innocent,
    Detective,
    Mafia,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Innocent => write!(f, "Innocent"),
            Role::Detective => write!(f, "Detective"),
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

    fn get_private_state(&self) -> String {
        let mut state = format!("Status: {}", self.status);
        for guess in self.hid_guess.iter() {
            state = format!("{}, {}", state, guess);
        }
        state
    }
}

fn read_input(input: String) -> Vec<f64> {
    let mut out = vec![];
    for num in input.split(",") {
        if let Ok(res) = num.parse::<f64>() {
            out.push(res);
        };
    }
    out
}

fn max_index<T>(input: &Vec<T>) -> Option<usize>
where
    T: PartialOrd,
{
    let mut max = None;
    for i in 0..input.len() {
        match max {
            Some(prev) => max = Some(if input[i] > input[prev] { i } else { prev }),
            None => max = Some(i),
        }
    }
    max
}

impl ConnectedPlayer<MafiaPlayer> {
    pub fn get_status(&self) -> Status {
        if let Some(player) = &self.player {
            return player.status;
        }
        Status::Dead
    }

    pub fn get_role(&self) -> Role {
        if let Some(player) = &self.player {
            return player.role;
        }
        Role::Innocent
    }
}
