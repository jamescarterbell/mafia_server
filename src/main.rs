#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

mod mafia;

fn main() {
    let game_list = mafia::GameList::new();
    rocket::ignite().manage(game_list).mount("/", routes![mafia::new_connection]).launch();
}
