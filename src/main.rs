#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod game;
mod mafia;

use game::{ConnectedPlayer, Connector, Game, Player};
use rand::seq::SliceRandom;

fn main() {
    game::launch::<MafiaPlayer, Mafia>();
}

pub struct Mafia {
    players: Vec<ConnectedPlayer<MafiaPlayer>>,
    phase: Phase,
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
                    if !player.check_connections() {
                        self.phase = Phase::End;
                        return;
                    }
                }

                // If all players are connected, create roles, and assign them to players
                let mut roles = self.create_role_vec();
                let mut rng = rand::thread_rng();
                roles.shuffle(&mut rng);

                for (i, player) in self.players.iter_mut().enumerate() {
                    player.player = Some(MafiaPlayer {
                        role: roles.pop().unwrap(),
                        status: Status::Alive,
                        id: i as u8,
                    });
                }
                self.phase = Phase::Detect;
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

#[derive(Clone)]
struct MafiaPlayer {
    role: Role,
    status: Status,
    id: u8,
}

#[derive(Clone)]
enum Status {
    Alive,
    Dead,
}

#[derive(Clone)]
enum Role {
    Innocent,
    Detective,
    Doctor,
    Mafia,
}

impl Player for MafiaPlayer {}
