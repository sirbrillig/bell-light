use bevy::prelude::*;
use bevy_behave::prelude::*;

use crate::{ai::AiSet, movement::IntendedVelocity};

#[derive(Component, Clone)]
pub struct StopMoving;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, stop_moving.in_set(AiSet::Behavior));
}

fn stop_moving(query: Query<&BehaveCtx, With<StopMoving>>, mut commands: Commands) {
    for ctx in query.iter() {
        commands
            .entity(ctx.target_entity())
            .insert(IntendedVelocity::x(0.0));
        commands.trigger(ctx.success());
    }
}
