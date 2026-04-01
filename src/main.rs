#![allow(dead_code)]

mod cli;
mod config;
mod export;
mod model;
mod parser;
mod store;
mod tui;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => cli::run(cmd),
        None => launch_tui(),
    }
}

fn launch_tui() {
    let db_path = config::db_path();
    std::fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create ~/.verchat/");

    let store = store::Store::open(&db_path).expect("Failed to open database");

    if let Err(e) = tui::run(store) {
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }
}
