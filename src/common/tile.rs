use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::message::NetworkId;

pub const TILE_SIZE: f32 = 50.0;

#[derive(Debug, Component, Clone, Copy, Serialize, Deserialize)]
pub enum TileKind {
    Stone,
    Grass,
}

#[derive(Debug, Component, Deref, DerefMut, Clone, Copy)]
pub struct GridPos(pub IVec2);

#[derive(Debug, Deref, DerefMut, Clone, Serialize, Deserialize)]
pub struct Tiles(pub HashMap<IVec2, (NetworkId, TileKind)>);

pub fn spawn_block(commands: &mut Commands, id: NetworkId, pos: IVec2, kind: TileKind) -> Entity {
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: match kind {
                    TileKind::Stone => Color::DARK_GRAY,
                    TileKind::Grass => Color::GREEN,
                },
                custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(
                pos.x as f32 * TILE_SIZE,
                pos.y as f32 * TILE_SIZE,
                0.0,
            )),
            ..default()
        })
        .insert(GridPos(pos))
        .insert(id)
        .insert(kind)
        .id()
}
