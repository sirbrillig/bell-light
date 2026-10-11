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
const SHAPE_HEIGHT: f32 = 32.;

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
        let hit = spatial_query.cast_shape(
            &Collider::rectangle(SHAPE_HEIGHT, SHAPE_WIDTH),
            enemy_pos.0,
            0.0,
            wait.direction.to_dir2(),
            &ShapeCastConfig::from_max_distance(wait.max_distance),
            &SpatialQueryFilter::from_mask([GameLayers::Player]),
        );
        if hit.is_some() {
            commands.trigger(ctx.success());
            commands.entity(task).insert(TaskReported);
        }
    }
}
