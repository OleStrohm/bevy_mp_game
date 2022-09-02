use std::collections::HashMap;

use bevy::prelude::*;
use bevy_renet::renet::RenetError;
use serde::{Deserialize, Serialize};

use crate::player::{PlayerLocation, PlayerSyncData};

pub const PROTOCOL_ID: u64 = 7;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientUnreliable {
    PlayerMovement(PlayerLocation),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerReliable {
    PlayerJoined(u64, PlayerSyncData),
    PlayerLeft(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerUnreliable {
    PlayerMoved(u64, PlayerLocation),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerBlocking {
    SyncPlayers(HashMap<u64, PlayerSyncData>),
}

pub fn panic_on_error(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}
