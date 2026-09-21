use avian2d::physics_transform::Position;
use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::{
    ai::{AiSet, tasks::TaskReported},
    player::Player,
};

#[derive(Component, Clone, Copy)]
pub struct WaitUntilPlayerIsNear;

#[derive(Component, Clone, Copy)]
pub struct DetectionDistance(pub f32);

pub fn plugin(app: &mut App) {
    app.add_systems(Update, wait_for_player.in_set(AiSet::Behavior));
}

fn wait_for_player(
    query: Query<(Entity, &BehaveCtx), (With<WaitUntilPlayerIsNear>, Without<TaskReported>)>,
    mut commands: Commands,
    player_pos: Single<&Position, With<Player>>,
    entities: Query<&Position>,
    mover_props: Query<&DetectionDistance>,
) {
    for (task, ctx) in query.iter() {
        let Ok(enemy_pos) = entities.get(ctx.target_entity()) else {
            continue;
        };
        let Ok(near_distance) = mover_props.get(ctx.target_entity()) else {
            continue;
        };
        let distance_to_player = player_pos.distance(enemy_pos.0);
        if distance_to_player <= near_distance.0 {
            commands.trigger(ctx.success());
            commands.entity(task).insert(TaskReported);
        }
    }
}
