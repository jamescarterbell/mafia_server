#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod mafia;

use std::sync::mpsc::channel;
use std::sync::Mutex;

fn main() {
    let (send_games, recieve_games) = channel();
    let (send_players, recieve_players) = channel();

    mafia::check_games(send_games, recieve_players);
    mafia::run_active_games(recieve_games, send_players.clone());
    rocket::ignite()
        .manage(Mutex::new(send_players))
        .mount("/", routes![mafia::new_connection])
        .launch();
}
