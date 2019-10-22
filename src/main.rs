#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod mafia;

use std::sync::Arc;
use std::thread;

fn main() {
    let game_list = Arc::new(mafia::GameList::new());
    let in_list = game_list.clone();
    let t = thread::spawn(move || loop {
        println!("{}", in_list.games.read().unwrap().len())
    });
    rocket::ignite()
        .manage(game_list)
        .mount("/", routes![mafia::new_connection])
        .launch();
}
