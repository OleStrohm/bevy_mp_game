use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_renet::renet::{ClientAuthentication, RenetClient, RenetConnectionConfig};
use bevy_renet::{run_if_client_connected, RenetClientPlugin};

use crate::common::message::{
    ClientUnreliable, NetworkEvent, NetworkId, NetworkIds, NetworkSpawnCommand, RenetClientExt,
    ServerBlocking, ServerReliable, ServerUnreliable, PROTOCOL_ID,
};
use crate::common::panic_on_error;
use crate::common::player::{Player, PlayerLocation};
use crate::common::tile::{spawn_block, GridPos, TileKind, Tiles, TILE_SIZE};
use crate::{log, multiplayer_role, MultiplayerRole};

#[derive(Default)]
struct Lobby {
    players: HashMap<u64, Entity>,
}

pub fn client() {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let config = RenetConnectionConfig::default();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        protocol_id: PROTOCOL_ID,
        client_id,
        server_addr,
        user_data: None,
    };

    let client = RenetClient::new(current_time, socket, client_id, config, authentication).unwrap();
    App::new()
        .init_resource::<MousePos>()
        .init_resource::<CurrentGridCoord>()
        .init_resource::<NetworkIds>()
        .insert_resource(ClearColor(Color::rgb(0.35, 0.1, 0.7)))
        .insert_resource(WindowDescriptor {
            title: if matches!(multiplayer_role(), MultiplayerRole::Client) {
                "Making a multiplayer game in Rust - Client".to_string()
            } else {
                "Making a multiplayer game in Rust".to_string()
            },
            width: 2560. / 2.4,
            height: 1440. / 2.4,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(RenetClientPlugin)
        .insert_resource(client)
        .insert_resource(Lobby::default())
        .add_startup_system(setup)
        .add_system(move_player)
        .add_system(send_player_pos_to_server.after(move_player))
        .add_system(receive_message_system.with_run_criteria(run_if_client_connected))
        .add_system(panic_on_error)
        .add_system(hit_tile)
        .add_system(update_mouse_pos)
        .add_system(spawn_tile_on_click)
        .run();
}

#[derive(Default, Deref, DerefMut)]
struct MousePos(Vec2);

#[derive(Default, Deref, DerefMut)]
struct CurrentGridCoord(IVec2);

fn update_mouse_pos(
    window: Res<Windows>,
    camera: Query<&Transform, With<Camera>>,
    mut mouse_pos: ResMut<MousePos>,
    mut grid_coord: ResMut<CurrentGridCoord>,
) {
    let window = window.get_primary().unwrap();
    let width = window.width();
    let height = window.height();
    let camera = camera.single().translation;
    if let Some(window_pos) = window.cursor_position() {
        mouse_pos.0 = Vec2::new(
            camera.x + window_pos.x - width / 2.,
            camera.x + window_pos.y - height / 2.,
        );
        grid_coord.0 = IVec2::new(
            (mouse_pos.x / TILE_SIZE).round() as i32,
            (mouse_pos.y / TILE_SIZE).round() as i32,
        );
    }
}

fn spawn_tile_on_click(
    mut client: ResMut<RenetClient>,
    input: Res<Input<MouseButton>>,
    tiles: Query<(&NetworkId, &GridPos), With<TileKind>>,
    grid_coord: Res<CurrentGridCoord>,
) {
    if input.just_pressed(MouseButton::Right) {
        if !tiles.iter().any(|(_, &GridPos(pos))| grid_coord.0 == pos) {
            client.send_event(NetworkEvent::SpawnBlock(grid_coord.0, TileKind::Stone));
        }
    }
}

fn hit_tile(
    mut client: ResMut<RenetClient>,
    input: Res<Input<MouseButton>>,
    tiles: Query<(&NetworkId, &GridPos), With<TileKind>>,
    grid_coord: Res<CurrentGridCoord>,
) {
    if input.just_pressed(MouseButton::Left) {
        for (&id, &GridPos(pos)) in &tiles {
            if grid_coord.0 == pos {
                client.send_event(NetworkEvent::BreakBlock(id));
            }
        }
    }
}

#[derive(Component)]
pub struct Remote;

fn setup(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    client: Res<RenetClient>,
    mut lobby: ResMut<Lobby>,
) {
    let window = windows.get_primary_mut().unwrap();

    match multiplayer_role() {
        crate::MultiplayerRole::Host => {
            window.set_position(IVec2::new(100, (1440. - 2. * 1440. / 2.5) as i32 / 4))
        }
        crate::MultiplayerRole::Client => window.set_position(IVec2::new(
            100,
            1400 / 2 + (1440. - 2. * 1440. / 2.5) as i32 / 4,
        )),
        _ => (),
    }

    commands.spawn_bundle(Camera2dBundle::default());

    let player = Player::create(&mut commands, Default::default(), false);
    lobby.players.insert(client.client_id(), player);
}

fn send_player_pos_to_server(
    mut player: Query<&Transform, (Changed<Transform>, With<Player>, Without<Remote>)>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(tf) = player.get_single_mut() {
        client.send(ClientUnreliable::PlayerMovement(PlayerLocation(
            tf.translation.xy(),
        )));
    }
}

fn move_player(
    mut player: Query<&mut Transform, (With<Player>, Without<Remote>)>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let speed = 500.0;
    let mut tf = player.single_mut();
    if input.pressed(KeyCode::W) {
        tf.translation.y += speed * time.delta_seconds();
    }
    if input.pressed(KeyCode::A) {
        tf.translation.x -= speed * time.delta_seconds();
    }
    if input.pressed(KeyCode::S) {
        tf.translation.y -= speed * time.delta_seconds();
    }
    if input.pressed(KeyCode::D) {
        tf.translation.x += speed * time.delta_seconds();
    }
}

fn receive_message_system(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<Lobby>,
    mut network_ids: ResMut<NetworkIds>,
    mut player_data: Query<(&mut Transform, &mut Sprite)>,
) {
    while let Some(message) = client.receive_message(0) {
        match bincode::deserialize(&message).unwrap() {
            ServerReliable::PlayerJoined(id, data) => {
                let new_player = Player::create(&mut commands, data, true);
                lobby.players.insert(id, new_player);
                log!("Client {} joined", id)
            }
            ServerReliable::PlayerLeft(id) => {
                let player = lobby.players.remove(&id).unwrap();
                commands.entity(player).despawn();
                log!("Client {} left", id)
            }
            ServerReliable::Event(event) => {
                log!("Got event {event:?}");
                match event {
                    NetworkEvent::SpawnBlock(_, _) => unreachable!("can't happen"),
                    NetworkEvent::BreakBlock(id) => {
                        commands.entity(network_ids.remove(&id).unwrap()).despawn();
                    }
                }
            }
            ServerReliable::Spawn(id, command) => match command {
                NetworkSpawnCommand::Block(pos, kind) => {
                    let tile = spawn_block(&mut commands, id, pos, kind);
                    network_ids.insert(id, tile);
                }
            },
        }
    }
    while let Some(message) = client.receive_message(1) {
        match bincode::deserialize(&message).unwrap() {
            ServerUnreliable::PlayerMoved(id, PlayerLocation(pos)) => {
                if let Some(player) = lobby.players.get(&id).copied() {
                    if let Ok((mut tf, _)) = player_data.get_mut(player) {
                        tf.translation.x = pos.x;
                        tf.translation.y = pos.y;
                    }
                }
            }
        }
    }
    while let Some(message) = client.receive_message(2) {
        match bincode::deserialize(&message).unwrap() {
            ServerBlocking::SyncPlayers(players) => {
                log!("syncing {} players", players.len());
                for (&client_id, &sync_data) in &players {
                    match lobby.players.get(&client_id) {
                        Some(ent) => {
                            if let Ok((mut tf, mut sprite)) = player_data.get_mut(*ent) {
                                tf.translation.x = sync_data.pos.x;
                                tf.translation.y = sync_data.pos.y;
                                sprite.color = sync_data.color;
                            }
                        }
                        None => {
                            lobby
                                .players
                                .insert(client_id, Player::create(&mut commands, sync_data, true));
                        }
                    }
                }
            }
            ServerBlocking::SyncWorld(tiles) => {
                log!("Received {} tiles", tiles.len());
                for (&pos, &(id, kind)) in &tiles {
                    let tile = spawn_block(&mut commands, id, pos, kind);
                    network_ids.insert(id, tile);
                }
                commands.insert_resource(Tiles(tiles));
            }
        }
    }
}
