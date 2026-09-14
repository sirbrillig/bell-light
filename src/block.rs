use crate::attack::HurtBoxBundle;
use crate::movement::*;
use avian2d::dynamics::rigid_body::{Friction, LockedAxes, RigidBody};
use bevy::{prelude::*, sprite::Anchor};
use bevy_ecs_ldtk::LdtkEntity;
use bevy_ecs_ldtk::app::LdtkEntityAppExt;

#[derive(Component, Default)]
pub struct Block;

#[derive(Component, Default)]
pub struct BlockHurtBox;

#[derive(Bundle, LdtkEntity)]
pub struct BlockBundle {
    block: Block,
    #[sprite_sheet("Stone_Node_Animation.png", 32, 32, 9, 1, 0, 0, 0)]
    sprite_sheet: Sprite,
    body: RigidBody,
    friction: Friction,
    axes: LockedAxes,
    anchor: Anchor,
}

impl Default for BlockBundle {
    fn default() -> Self {
        Self {
            block: Block,
            sprite_sheet: Sprite::default(),
            body: RigidBody::Dynamic,
            friction: Friction::ZERO
                .with_combine_rule(avian2d::dynamics::rigid_body::CoefficientCombine::Min),
            axes: LockedAxes::ROTATION_LOCKED,
            anchor: Anchor(Vec2::new(0., -0.25)),
        }
    }
}

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_spawned);
        app.register_ldtk_entity::<BlockBundle>("Block");
    }
}

fn on_spawned(event: On<Add, Block>, mut commands: Commands) {
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            // @todo let these blocks be spawned where they are positioned instead of above the
            // ground
            EnvColliderBundle::new(
                GameLayers::Props,
                [
                    GameLayers::Environment,
                    GameLayers::Enemies,
                    GameLayers::Player,
                ],
                15.5,
                15.5,
            ),
        ));
    });
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            BlockHurtBox,
            HurtBoxBundle::new(
                GameLayers::EnemyHurtBox,
                GameLayers::PlayerHurtBox,
                16.,
                16.,
            ),
        ));
    });
}
