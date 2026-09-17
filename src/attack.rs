use avian2d::collision::collider::{Collider, CollidingEntities, CollisionLayers, LayerMask};
use avian2d::collision::{collider::Sensor, collision_events::CollisionEventsEnabled};
use bevy::prelude::*;
use bevy::{ecs::component::Component, time::Timer};

use crate::movement::GameLayers;

#[derive(Component)]
pub struct Attacking {
    pub timer: Timer,
    pub hitbox: Option<Entity>,
}

#[derive(Component)]
#[require(Sensor, CollisionEventsEnabled)]
pub struct HurtBox;

#[derive(Component)]
#[require(Sensor, CollisionEventsEnabled)]
pub struct HitBox;

#[derive(Bundle)]
pub struct HitBoxBundle {
    pub hitbox: HitBox,
    pub layers: CollisionLayers,
    pub collider: Collider,
}

impl HitBoxBundle {
    pub fn new(
        layer: GameLayers,
        other_layers: impl Into<LayerMask>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            hitbox: HitBox,
            layers: CollisionLayers::new(layer, other_layers),
            collider: Collider::rectangle(width, height),
        }
    }
}

#[derive(Bundle)]
pub struct HurtBoxBundle {
    pub hurtbox: HurtBox,
    pub layers: CollisionLayers,
    pub collider: Collider,
    pub entities: CollidingEntities,
}

impl HurtBoxBundle {
    pub fn new(
        layer: GameLayers,
        other_layers: impl Into<LayerMask>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            hurtbox: HurtBox,
            layers: CollisionLayers::new(layer, other_layers),
            collider: Collider::rectangle(width, height),
            entities: CollidingEntities::default(),
        }
    }
}
