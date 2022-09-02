use std::collections::HashMap;

use bevy::prelude::*;
use bevy_renet::renet::RenetError;
use serde::{Deserialize, Serialize};

use crate::player::{PlayerLocation, PlayerSyncData};

pub const PROTOCOL_ID: u64 = 7;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    PlayerMovement(PlayerLocation),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    SyncPlayers(HashMap<u64, PlayerSyncData>),
    PlayerJoined(u64, PlayerSyncData),
    PlayerLeft(u64),
    PlayerMoved(u64, PlayerLocation),
}

pub fn panic_on_error(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}
