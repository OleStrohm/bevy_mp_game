use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::client::Remote;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerLocation(pub Vec2);

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerSyncData {
    pub pos: Vec2,
    pub color: Color,
}

#[derive(Component)]
pub struct Player;

impl Player {
    pub fn create(commands: &mut Commands, data: PlayerSyncData, remote: bool) -> Entity {
        let player = commands
            .spawn()
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    color: data.color,
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: data.pos.extend(0.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Player)
            .id();

        if remote {
            commands.entity(player).insert(Remote);
        }

        player
    }
}
