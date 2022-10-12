use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_renet::renet::{
    RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent,
};
use bevy_renet::RenetServerPlugin;

use crate::common::message::{
    ClientReliable, ClientUnreliable, NetworkEvent, NetworkId, NetworkSpawnCommand, RenetServerExt,
    ServerBlocking, ServerReliable, ServerUnreliable, PROTOCOL_ID,
};
use crate::common::panic_on_error;
use crate::common::player::{PlayerLocation, PlayerSyncData};
use crate::common::tile::{GridPos, TileKind, Tiles};
use crate::log;

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
        .add_startup_system(create_world)
        .add_system(receive_message_system)
        .add_system(handle_events_system)
        .add_system(panic_on_error)
        .add_system(update_world)
        .run();
}

fn create_block(commands: &mut Commands, pos: IVec2, kind: TileKind) -> NetworkId {
    NetworkId(commands.spawn().insert(GridPos(pos)).insert(kind).id())
}

fn create_world(mut commands: Commands) {
    let mut tiles = HashMap::new();
    for y in 0..5 {
        for x in 0..5 {
            let pos = IVec2::new(x, y);
            let kind = if (x + y) % 2 == 0 {
                TileKind::Stone
            } else {
                TileKind::Grass
            };
            let id = create_block(&mut commands, pos, kind);
            tiles.insert(pos, (id, kind));
        }
    }
    commands.insert_resource(Tiles(tiles));
}

fn update_world(
    mut tiles: ResMut<Tiles>,
    added_tiles: Query<(Entity, &TileKind, &GridPos), Added<TileKind>>,
) {
    for (entity, &kind, &pos) in &added_tiles {
        tiles.insert(*pos, (NetworkId(entity), kind));
    }
}

fn receive_message_system(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<Lobby>,
    mut tiles: ResMut<Tiles>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, 0) {
            match bincode::deserialize(&message).unwrap() {
                ClientReliable::Event(event) => match event {
                    NetworkEvent::SpawnBlock(pos, kind) => {
                        let tile = NetworkId(commands.spawn().id());

                        server.broadcast(ServerReliable::Spawn(
                            tile,
                            NetworkSpawnCommand::Block(pos, kind),
                        ))
                    }
                    NetworkEvent::BreakBlock(id) => {
                        commands.entity(*id).despawn();
                        tiles.retain(|_, &mut (tile_id, _)| tile_id != id);
                        server.broadcast_event(event);
                    }
                },
            }
        }
        while let Some(message) = server.receive_message(client_id, 1) {
            match bincode::deserialize(&message).unwrap() {
                ClientUnreliable::PlayerMovement(loc @ PlayerLocation(pos)) => {
                    lobby.players.get_mut(&client_id).unwrap().pos = pos;

                    server
                        .broadcast_except(client_id, ServerUnreliable::PlayerMoved(client_id, loc));
                }
            }
        }
    }
}

fn handle_events_system(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<Lobby>,
    tiles: Res<Tiles>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _user_data) => {
                let player_data = PlayerSyncData {
                    pos: Vec2::new(rand::random::<f32>() * 300.0, rand::random::<f32>() * 300.0),
                    color: Color::rgb(rand::random(), rand::random(), rand::random()),
                };
                lobby.players.insert(*id, player_data);

                server.send_to(*id, ServerBlocking::SyncPlayers(lobby.players.clone()));
                server.send_to(*id, ServerBlocking::SyncWorld(tiles.0.clone()));
                server.broadcast_except(*id, ServerReliable::PlayerJoined(*id, player_data));
                log!("Client {} connected", id);
            }
            ServerEvent::ClientDisconnected(id) => {
                lobby.players.remove(id);
                server.broadcast_except(*id, ServerReliable::PlayerLeft(*id));
                log!("Client {} disconnected", id);
            }
        }
    }
}
