use crate::animation::{AnimatedSpriteBundle, AnimationKey, AnimationSet, CharacterAnimationClip};
use crate::attack::{HurtBox, HurtBoxBundle};
use crate::{GameSet, movement::*};
use avian2d::collision::collider::CollidingEntities;
use avian2d::collision::collider::collider_hierarchy::ColliderOf;
use avian2d::dynamics::rigid_body::{LockedAxes, RigidBody};
use bevy::prelude::*;
use bevy_ecs_ldtk::LdtkEntity;
use bevy_ecs_ldtk::app::LdtkEntityAppExt;
use std::collections::HashMap;

#[derive(Component, Default)]
pub struct Block;

#[derive(Component, Default)]
pub struct BlockHurtBox;

#[derive(Bundle, LdtkEntity)]
pub struct BlockBundle {
    block: Block,
    animation: AnimatedSpriteBundle,
    body: RigidBody,
    axes: LockedAxes,
}

const BLOCK_BREAK_TIME: f32 = 0.3;
const BLOCK_BREAK_FRAMES: usize = 9;

impl Default for BlockBundle {
    fn default() -> Self {
        Self {
            block: Block,
            animation: AnimatedSpriteBundle::new(
                -0.25,
                BLOCK_BREAK_FRAMES,
                BLOCK_BREAK_TIME / BLOCK_BREAK_FRAMES as f32,
            ),
            body: RigidBody::Dynamic,
            axes: LockedAxes::ROTATION_LOCKED,
        }
    }
}

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_block);
        app.add_systems(
            Update,
            (detect_hit, destroy_block).in_set(GameSet::Reactions),
        );
        app.add_observer(on_spawned);
        app.add_observer(on_breaking);
        app.register_ldtk_entity::<BlockBundle>("Block");
    }
}

fn on_spawned(event: On<Add, Block>, animations: Res<BlockAnimations>, mut commands: Commands) {
    commands.entity(event.entity).insert(animations.0.clone());
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            Transform::from_xyz(0., 0.05, 0.),
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
            HurtBoxBundle::new(GameLayers::Props, GameLayers::PlayerPowerBox, 13.5, 13.5),
        ));
    });
}

fn on_breaking(event: On<Add, BlockBreaking>, mut commands: Commands) {
    commands.entity(event.entity).insert(AnimationKey::Shatter);
}

#[derive(Resource)]
pub struct BlockAnimations(AnimationSet);

fn setup_block(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
) {
    let idle = CharacterAnimationClip {
        image: asset_server.load("Stone_Node_Animation.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(32),
            9,
            1,
            None,
            None,
        )),
        frames: 1,
    };
    let clip = CharacterAnimationClip {
        image: asset_server.load("Stone_Node_Animation.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(32),
            9,
            1,
            None,
            None,
        )),
        frames: 9,
    };
    commands.insert_resource(BlockAnimations(AnimationSet {
        animation_map: HashMap::from([(AnimationKey::Idle, idle), (AnimationKey::Shatter, clip)]),
    }));
}

#[derive(Component, Default)]
pub struct BlockBreaking {
    pub timer: Timer,
}

fn detect_hit(
    query: Query<(&CollidingEntities, &ColliderOf), (With<HurtBox>, With<BlockHurtBox>)>,
    mut commands: Commands,
) {
    for (hurtbox, block) in query.iter() {
        if !hurtbox.is_empty() {
            commands.entity(block.body).insert_if_new(BlockBreaking {
                timer: Timer::from_seconds(BLOCK_BREAK_TIME, TimerMode::Once),
            });
        }
    }
}

fn destroy_block(
    mut query: Query<(Entity, &mut BlockBreaking)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (block, mut block_break) in query.iter_mut() {
        // The actual effect timer; animation timer is run by animation module
        block_break.timer.tick(time.delta());
        if block_break.timer.is_finished() {
            commands.entity(block).despawn();
        }
    }
}
