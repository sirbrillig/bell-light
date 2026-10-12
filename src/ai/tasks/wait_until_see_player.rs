use avian2d::{
    collision::collider::Collider,
    physics_transform::Position,
    spatial_query::{ShapeCastConfig, SpatialQuery, SpatialQueryFilter},
};
use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::{
    ai::{AiSet, tasks::TaskReported},
    movement::{GameLayers, OrthagonalDirection},
    player::PlayerEnvBox,
};

#[derive(Component, Clone, Copy)]
pub struct WaitUntilSeePlayer {
    pub direction: OrthagonalDirection,
    pub max_distance: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, wait_for_player.in_set(AiSet::Behavior));
}

const SHAPE_WIDTH: f32 = 32.;

fn wait_for_player(
    query: Query<(Entity, &BehaveCtx, &WaitUntilSeePlayer), Without<TaskReported>>,
    player_colliders: Query<(), With<PlayerEnvBox>>,
    mut commands: Commands,
    entities: Query<&Position>,
    spatial_query: SpatialQuery,
) {
    for (task, ctx, wait) in query.iter() {
        let Ok(enemy_pos) = entities.get(ctx.target_entity()) else {
            continue;
        };
        let direction_vec = wait.direction.to_dir2();
        // Move position in front of the origin so it does not collide with things behind it.
        let shapecast_origin = enemy_pos.0 + (direction_vec * ((SHAPE_WIDTH / 2.) + 2.));
        let Some(hit) = spatial_query.cast_shape(
            // If this ever needs to be non-square, we need to rotate.
            &Collider::rectangle(SHAPE_WIDTH, SHAPE_WIDTH),
            // Move position in front of the origin so it does not collide with things behind it.
            shapecast_origin,
            // rotation is 0 because have a square.
            0.0,
            direction_vec,
            &ShapeCastConfig::from_max_distance(wait.max_distance),
            &SpatialQueryFilter::from_mask([
                GameLayers::PlayerEnvBox,
                GameLayers::Environment,
                GameLayers::Platforms,
                GameLayers::Props,
            ]),
        ) else {
            continue;
        };
        if player_colliders.contains(hit.entity) {
            commands.trigger(ctx.success());
            commands.entity(task).insert(TaskReported);
        }
    }
}
