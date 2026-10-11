use avian2d::{
    physics_transform::Position,
    spatial_query::{SpatialQuery, SpatialQueryFilter},
};
use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::{
    ai::{AiSet, tasks::TaskReported},
    movement::{GameLayers, OrthagonalDirection},
};

#[derive(Component, Clone, Copy)]
pub struct WaitUntilSeePlayer {
    pub direction: OrthagonalDirection,
    pub max_distance: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, wait_for_player.in_set(AiSet::Behavior));
}

fn wait_for_player(
    query: Query<(Entity, &BehaveCtx, &WaitUntilSeePlayer), Without<TaskReported>>,
    mut commands: Commands,
    entities: Query<&Position>,
    spatial_query: SpatialQuery,
) {
    for (task, ctx, wait) in query.iter() {
        let Ok(enemy_pos) = entities.get(ctx.target_entity()) else {
            continue;
        };
        // @todo let ray be interrupted by environment without counting as a detection
        // @todo use shapecasting
        let hit = spatial_query.cast_ray(
            enemy_pos.0,
            wait.direction.to_vec2(),
            wait.max_distance,
            false,
            &SpatialQueryFilter::from_mask([GameLayers::Player]),
        );
        if hit.is_some() {
            commands.trigger(ctx.success());
            commands.entity(task).insert(TaskReported);
        }
    }
}
