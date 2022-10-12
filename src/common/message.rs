use std::collections::HashMap;

use bevy::prelude::*;
use bevy_renet::renet::{RenetClient, RenetServer};
use serde::{Deserialize, Serialize};

use super::player::{PlayerLocation, PlayerSyncData};
use super::tile::TileKind;

pub const PROTOCOL_ID: u64 = 7;

#[derive(
    Component, Debug, Deref, DerefMut, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash,
)]
pub struct NetworkId(pub Entity);

#[derive(Debug, Deref, DerefMut, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NetworkIds(HashMap<NetworkId, Entity>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkEvent {
    SpawnBlock(IVec2, TileKind),
    BreakBlock(NetworkId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkSpawnCommand {
    Block(IVec2, TileKind),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientReliable {
    Event(NetworkEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientUnreliable {
    PlayerMovement(PlayerLocation),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerReliable {
    PlayerJoined(u64, PlayerSyncData),
    PlayerLeft(u64),
    Event(NetworkEvent),
    Spawn(NetworkId, NetworkSpawnCommand),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerUnreliable {
    PlayerMoved(u64, PlayerLocation),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerBlocking {
    SyncPlayers(HashMap<u64, PlayerSyncData>),
    SyncWorld(HashMap<IVec2, (NetworkId, TileKind)>),
}

pub trait SendOverRenet {
    const CHANNEL_ID: u8;
    fn prepare(&self) -> Vec<u8>;
}

pub trait RenetClientExt {
    fn send<Msg: SendOverRenet>(&mut self, msg: Msg);

    fn send_event(&mut self, event: NetworkEvent) {
        self.send(ClientReliable::Event(event));
    }
}

impl RenetClientExt for RenetClient {
    fn send<Msg: SendOverRenet>(&mut self, msg: Msg) {
        self.send_message(Msg::CHANNEL_ID, msg.prepare());
    }
}

pub trait RenetServerExt {
    fn send_to<Msg: SendOverRenet>(&mut self, client_id: u64, msg: Msg);
    fn broadcast<Msg: SendOverRenet>(&mut self, msg: Msg);
    fn broadcast_except<Msg: SendOverRenet>(&mut self, client_id: u64, msg: Msg);

    fn broadcast_event(&mut self, event: NetworkEvent) {
        self.broadcast(ServerReliable::Event(event));
    }
}

impl RenetServerExt for RenetServer {
    fn send_to<Msg: SendOverRenet>(&mut self, client_id: u64, msg: Msg) {
        self.send_message(client_id, Msg::CHANNEL_ID, msg.prepare());
    }

    fn broadcast<Msg: SendOverRenet>(&mut self, msg: Msg) {
        self.broadcast_message(Msg::CHANNEL_ID, msg.prepare());
    }

    fn broadcast_except<Msg: SendOverRenet>(&mut self, client_id: u64, msg: Msg) {
        self.broadcast_message_except(client_id, Msg::CHANNEL_ID, msg.prepare());
    }
}

impl SendOverRenet for ClientReliable {
    const CHANNEL_ID: u8 = 0;

    fn prepare(&self) -> Vec<u8> {
        bincode::serialize(self).expect("This message is always serializable")
    }
}

impl SendOverRenet for ClientUnreliable {
    const CHANNEL_ID: u8 = 1;

    fn prepare(&self) -> Vec<u8> {
        bincode::serialize(self).expect("This message is always serializable")
    }
}

impl SendOverRenet for ServerReliable {
    const CHANNEL_ID: u8 = 0;

    fn prepare(&self) -> Vec<u8> {
        bincode::serialize(self).expect("This message is always serializable")
    }
}

impl SendOverRenet for ServerUnreliable {
    const CHANNEL_ID: u8 = 1;

    fn prepare(&self) -> Vec<u8> {
        bincode::serialize(self).expect("This message is always serializable")
    }
}

impl SendOverRenet for ServerBlocking {
    const CHANNEL_ID: u8 = 2;

    fn prepare(&self) -> Vec<u8> {
        bincode::serialize(self).expect("This message is always serializable")
    }
}
