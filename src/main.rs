mod client;
mod event;
mod scheduler;

use clap::{Arg, ArgAction, Command};
use client::Client;
use log::info;
use rusqlite::Connection;
use scheduler::Scheduler;

#[tokio::main]
async fn main() -> Result<(), String> {
    env_logger::init();

    let args = Command::new("NotifyMe")
        .version("1.0")
        .arg(
            Arg::new("client")
                .short('c')
                .long("client")
                .help("Execute as client")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let conn = Connection::open("notify_me.db").unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            message TEXT NOT NULL,
            recurrence_pattern TEXT,
            date TEXT NOT NULL,
            deleted_at TEXT DEFAULT NULL
        )",
        (),
    )
    .unwrap();

    if !args.get_flag("client") {
        info!("Starting scheduler");
        let scheduler = Scheduler::new(&conn);

        scheduler.start().await;
    }

    let client = Client::new(&conn);
    client.start();

    Ok(())
}
