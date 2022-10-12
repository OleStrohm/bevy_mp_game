use bevy::prelude::EventReader;
use bevy_renet::renet::RenetError;

pub mod tile;
pub mod message;
pub mod player;

pub fn panic_on_error(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}
