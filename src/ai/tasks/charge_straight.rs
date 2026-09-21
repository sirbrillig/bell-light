use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::{
    ai::{AiSet, tasks::TaskReported},
    enemies::Enemy,
    movement::{IntendedVelocity, MovementSpeed, OrthagonalDirection},
};

#[derive(Component, Clone, PartialEq)]
pub struct ChargeDirection(pub OrthagonalDirection);

#[derive(Component, Clone)]
pub struct ChargeStraight {
    pub direction: OrthagonalDirection,
}

impl From<&ChargeDirection> for ChargeStraight {
    fn from(value: &ChargeDirection) -> Self {
        Self { direction: value.0 }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, move_toward_entity.in_set(AiSet::Behavior));
}

fn move_toward_entity(
    query: Query<(&ChargeStraight, &BehaveCtx), Without<TaskReported>>,
    mut commands: Commands,
    mover_props: Query<&MovementSpeed, With<Enemy>>,
) {
    for (task, ctx) in query.iter() {
        let Ok(speed) = mover_props.get(ctx.target_entity()) else {
            continue;
        };

        let speed_vec = match task.direction {
            OrthagonalDirection::Up => Vec2::new(0., speed.0),
            OrthagonalDirection::Down => Vec2::new(0., -speed.0),
            OrthagonalDirection::Right => Vec2::new(speed.0, 0.),
            OrthagonalDirection::Left => Vec2::new(-speed.0, 0.),
        };
        commands
            .entity(ctx.target_entity())
            .insert(IntendedVelocity::from_vec(speed_vec));
    }
}
