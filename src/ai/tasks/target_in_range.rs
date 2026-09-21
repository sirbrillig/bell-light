use avian2d::physics_transform::Position;
use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::ai::{
    AiSet,
    tasks::{TaskReported, move_toward_entity::ChaseTarget},
};

#[derive(Component, Clone)]
pub struct TargetInRange {
    pub range: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, action.in_set(AiSet::Behavior));
}

fn action(
    query: Query<(Entity, &TargetInRange, &BehaveCtx), Without<TaskReported>>,
    mut commands: Commands,
    entities: Query<&Position>,
    mover_props: Query<(&ChaseTarget, &Position)>,
) {
    for (task, range, ctx) in query.iter() {
        let Ok((target, mover_pos)) = mover_props.get(ctx.target_entity()) else {
            continue;
        };
        let Ok(target_pos) = entities.get(target.0) else {
            continue;
        };

        let distance_to_player = target_pos.distance(mover_pos.0);
        if distance_to_player <= range.range {
            commands.trigger(ctx.success());
        } else {
            commands.trigger(ctx.failure());
        }
        commands.entity(task).insert(TaskReported);
    }
}
