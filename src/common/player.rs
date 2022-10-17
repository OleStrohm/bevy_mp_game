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
        let mut player = commands.spawn();
        player
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    color: data.color,
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: data.pos.extend(0.1),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Player);

        if remote {
            player.insert(Remote);
        } else {
            player.with_children(|commands| {
                commands.spawn().insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::GOLD,
                        custom_size: Some(Vec2::new(10.0, 10.0)),
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 0.5),
                    ..default()
                });
            });
        }

        player.id()
    }
}
