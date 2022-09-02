use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_renet::renet::{ClientAuthentication, RenetClient, RenetConnectionConfig};
use bevy_renet::{run_if_client_connected, RenetClientPlugin};

use crate::message::{
    panic_on_error, ClientUnreliable, ServerBlocking, ServerReliable, ServerUnreliable, PROTOCOL_ID,
};
use crate::player::{Player, PlayerLocation};
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
        .insert_resource(ClearColor(Color::rgb(0.35, 0.1, 0.7)))
        .insert_resource(WindowDescriptor {
            title: if matches!(multiplayer_role(), MultiplayerRole::Client) {
                "Making a multiplayer game in Rust - Client".to_string()
            } else {
                "Making a multiplayer game in Rust".to_string()
            },
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
        .run();
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
        crate::MultiplayerRole::Host => window.set_position(IVec2::new(
            (2560 - window.width() as i32) / 2,
            1440 - window.height() as i32,
        )),
        crate::MultiplayerRole::Client => window.set_position(IVec2::new(
            (1920 - window.width() as i32) / 2 + 2560,
            (1080 - window.height() as i32) / 2,
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
        let msg = ClientUnreliable::PlayerMovement(PlayerLocation(tf.translation.xy()));
        client.send_message(1, bincode::serialize(&msg).unwrap());
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
        }
    }
    while let Some(message) = client.receive_message(1) {
        match bincode::deserialize(&message).unwrap() {
            ServerUnreliable::PlayerMoved(id, PlayerLocation(pos)) => {
                let player = lobby.players.get(&id).copied().unwrap();
                let (mut tf, _) = player_data.get_mut(player).unwrap();
                tf.translation.x = pos.x;
                tf.translation.y = pos.y;
                //log!("Client {} is now at {}", id, pos)
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
                            let (mut tf, mut sprite) = player_data.get_mut(*ent).unwrap();
                            tf.translation.x = sync_data.pos.x;
                            tf.translation.y = sync_data.pos.y;
                            sprite.color = sync_data.color;
                        }
                        None => {
                            lobby
                                .players
                                .insert(client_id, Player::create(&mut commands, sync_data, true));
                        }
                    }
                }
            }
        }
    }
}
