#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod mafia;

use std::sync::mpsc::channel;
use std::sync::Arc;

fn main() {
    let waiting_games = Arc::new(mafia::GameList::new());
    let active_games = mafia::GameList::new();
    let in_list = waiting_games.clone();

    let (send, recieve) = channel();

    mafia::check_games(in_list, send);
    mafia::run_active_games(active_games, recieve);
    rocket::ignite()
        .manage(waiting_games)
        .mount("/", routes![mafia::new_connection])
        .launch();
}
