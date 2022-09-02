use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_renet::renet::{
    RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent,
};
use bevy_renet::RenetServerPlugin;

use crate::log;
use crate::message::{
    panic_on_error, ClientUnreliable, ServerBlocking, ServerReliable, ServerUnreliable, PROTOCOL_ID,
};
use crate::player::{PlayerLocation, PlayerSyncData};

#[derive(Default)]
struct Lobby {
    players: HashMap<u64, PlayerSyncData>,
}

pub fn server() {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind(server_addr).unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let connection_config = RenetConnectionConfig::default();
    let server_config =
        ServerConfig::new(64, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure);

    let server = RenetServer::new(current_time, server_config, connection_config, socket).unwrap();

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugin(RenetServerPlugin)
        .insert_resource(server)
        .insert_resource(Lobby::default())
        .add_system(receive_message_system)
        .add_system(handle_events_system)
        .add_system(panic_on_error)
        .run();
}

fn receive_message_system(mut server: ResMut<RenetServer>, mut lobby: ResMut<Lobby>) {
    let channel_id = 1;
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, channel_id) {
            match bincode::deserialize(&message).unwrap() {
                ClientUnreliable::PlayerMovement(loc @ PlayerLocation(pos)) => {
                    let msg = ServerUnreliable::PlayerMoved(client_id, loc);
                    lobby.players.get_mut(&client_id).unwrap().pos = pos;

                    server.broadcast_message_except(
                        client_id,
                        channel_id,
                        bincode::serialize(&msg).unwrap(),
                    );
                }
            }
        }
    }
}

fn handle_events_system(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<Lobby>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _user_data) => {
                let player_data = PlayerSyncData {
                    pos: Vec2::new(rand::random::<f32>() * 300.0, rand::random::<f32>() * 300.0),
                    color: Color::rgb(rand::random(), rand::random(), rand::random()),
                };
                lobby.players.insert(*id, player_data);

                let sync_msg = ServerBlocking::SyncPlayers(lobby.players.clone());
                server.send_message(*id, 2, bincode::serialize(&sync_msg).unwrap());
                let join_msg = ServerReliable::PlayerJoined(*id, player_data);
                server.broadcast_message_except(*id, 0, bincode::serialize(&join_msg).unwrap());
                log!("Client {} connected", id);
            }
            ServerEvent::ClientDisconnected(id) => {
                lobby.players.remove(id);
                let msg = ServerReliable::PlayerLeft(*id);
                server.broadcast_message_except(*id, 0, bincode::serialize(&msg).unwrap());
                log!("Client {} disconnected", id);
            }
        }
    }
}
