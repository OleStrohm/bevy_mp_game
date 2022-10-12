use std::fmt::Display;
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};

use owo_colors::OwoColorize;

use self::client::client;
use self::server::server;

mod client;
mod common;
mod server;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum MultiplayerRole {
    Host = 0,
    Client = 1,
    Server = 2,
}
static MULTIPLAYER_ROLE: AtomicU8 = AtomicU8::new(MultiplayerRole::Host as u8);

#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {{
        print!("[{}]: ", $crate::speaker());
        println!($($tt)*);
    }};
}

pub fn speaker() -> impl Display {
    match multiplayer_role() {
        MultiplayerRole::Host => "host".green().to_string(),
        MultiplayerRole::Client => "client".blue().to_string(),
        MultiplayerRole::Server => "server".yellow().to_string(),
    }
}

pub fn multiplayer_role() -> MultiplayerRole {
    match MULTIPLAYER_ROLE.load(Ordering::Relaxed) {
        0 => MultiplayerRole::Host,
        1 => MultiplayerRole::Client,
        2 => MultiplayerRole::Server,
        _ => unreachable!("Invalid value for multiplayer role"),
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("server") => {
            MULTIPLAYER_ROLE.store(MultiplayerRole::Server as u8, Ordering::Relaxed);
            server();
        }
        Some("client") => {
            MULTIPLAYER_ROLE.store(MultiplayerRole::Client as u8, Ordering::Relaxed);
            client()
        }
        Some("host") | None => {
            MULTIPLAYER_ROLE.store(MultiplayerRole::Host as u8, Ordering::Relaxed);
            let mut server = Command::new(std::env::args().nth(0).unwrap())
                .arg("server")
                .spawn()
                .unwrap();
            let mut player2 = Command::new(std::env::args().nth(0).unwrap())
                .arg("client")
                .spawn()
                .unwrap();
            client();
            player2.kill().unwrap();
            server.kill().unwrap();
        }
        _ => panic!("The first argument is nonsensical"),
    }
}
