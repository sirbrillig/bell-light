use bevy::{app::App, ecs::component::Component};

pub mod attack;
pub mod charge_straight;
pub mod face_target;
pub mod is_facing_target;
pub mod move_toward_entity;
pub mod stop_moving;
pub mod target_in_range;
pub mod wait_until_player_is_near;

/// Attach this to a task entity that runs on Update when it is complete to avoid double completing
#[derive(Component)]
pub struct TaskReported;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        wait_until_player_is_near::plugin,
        move_toward_entity::plugin,
        stop_moving::plugin,
        target_in_range::plugin,
        attack::plugin,
        is_facing_target::plugin,
        face_target::plugin,
        charge_straight::plugin,
    ));
}
