use avian2d::physics_transform::Position;
use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::{
    ai::{AiSet, tasks::TaskReported},
    movement::{FacingDirection, IntendedVelocity, MovementSpeed},
};

#[derive(Component, Clone)]
pub struct MoveTowardEntity {
    pub near_distance: f32,
    pub far_distance: f32,
}

#[derive(Component)]
pub struct ChaseTarget(pub Entity);

pub fn plugin(app: &mut App) {
    app.add_systems(Update, move_toward_entity.in_set(AiSet::Behavior));
}

fn move_toward_entity(
    query: Query<(Entity, &MoveTowardEntity, &BehaveCtx), Without<TaskReported>>,
    mut commands: Commands,
    entities: Query<&Position>,
    mover_props: Query<(&ChaseTarget, &MovementSpeed, &Position)>,
) {
    for (task, distance, ctx) in query.iter() {
        let Ok((target, speed, mover_pos)) = mover_props.get(ctx.target_entity()) else {
            continue;
        };
        let Ok(target_pos) = entities.get(target.0) else {
            continue;
        };
        let direction = if target_pos.x > mover_pos.x {
            FacingDirection::Right
        } else {
            FacingDirection::Left
        };

        let distance_to_player = target_pos.distance(mover_pos.0);
        if distance_to_player <= distance.near_distance {
            // Stop when we get close
            commands
                .entity(ctx.target_entity())
                .insert(IntendedVelocity::x(0.0));
            commands.trigger(ctx.success());
            commands.entity(task).insert(TaskReported);
        } else if distance_to_player >= distance.far_distance {
            // Stop if we get too far
            commands
                .entity(ctx.target_entity())
                .insert(IntendedVelocity::x(0.0));
            commands.trigger(ctx.success());
            commands.entity(task).insert(TaskReported);
        } else {
            // Otherwise move toward target
            commands
                .entity(ctx.target_entity())
                .insert(IntendedVelocity::x(match direction {
                    FacingDirection::Left => -speed.0,
                    FacingDirection::Right => speed.0,
                }));
        }
    }
}
