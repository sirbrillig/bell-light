use crate::ai::tasks::move_toward_entity::ChaseTarget;
use crate::animation::AnimatedSpriteBundle;
use crate::attack::HitBoxBundle;
use crate::movement::*;
use crate::player::Player;
use avian2d::spatial_query::SpatialQueryFilter;
use avian2d::{
    collision::collider::Collider,
    dynamics::rigid_body::{Friction, LockedAxes, RigidBody},
    spatial_query::ShapeCaster,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::LdtkEntity;

pub mod orc;

// These are defaults, they will probably need to be overridden
const ENEMY_HEIGHT: f32 = 16.0;
const ENEMY_HEIGHT_ANCHOR_OFFSET: f32 = 0.01;
const ENEMY_FOOT_HEIGHT: f32 = 2.0;
const ENEMY_FOOT_ANCHOR: f32 = -(ENEMY_HEIGHT / 2.) + (ENEMY_FOOT_HEIGHT / 2.);
const ENEMY_FOOT_RANGE: f32 = 2.0;

#[derive(Component, Default)]
pub struct Enemy;

#[derive(Component, Default)]
pub struct EnemyHurtBox;

#[derive(Component)]
pub struct HurtsWhenTouched {
    width: f32,
    height: f32,
}

#[derive(Bundle, LdtkEntity)]
pub struct EnemyCoreBundle {
    enemy: Enemy,
    state: MovementState,
    body: RigidBody,
    friction: Friction,
    sprite_height: EnemySpriteHeight,
    speed: MovementSpeed,
    intended_x_vel: IntendedXVelocity,
    ground_detection: GroundDetection,
    ground_detector: ShapeCaster,
    axes: LockedAxes,
    animation: AnimatedSpriteBundle,
    facing: FacingDirection,
}

#[derive(Component, Default, Clone, Copy)]
struct EnemySpriteHeight(f32);

pub struct EnemySettings {
    sprite_height: f32,
    sprite_height_offset: f32,
    speed: f32,
    ground_detector_height: f32,
    ground_detector_anchor: f32,
    ground_detector_range: f32,
    animation_default_frames: usize,
}

impl Default for EnemySettings {
    fn default() -> Self {
        EnemySettings {
            sprite_height: ENEMY_HEIGHT,
            sprite_height_offset: ENEMY_HEIGHT_ANCHOR_OFFSET,
            speed: 25.0,
            ground_detector_height: ENEMY_FOOT_HEIGHT,
            ground_detector_anchor: ENEMY_FOOT_ANCHOR,
            ground_detector_range: ENEMY_FOOT_RANGE,
            animation_default_frames: 6,
        }
    }
}

impl EnemyCoreBundle {
    pub fn with_settings(settings: EnemySettings) -> Self {
        Self {
            speed: MovementSpeed(settings.speed),
            ground_detector: ShapeCaster::with_query_filter(
                ShapeCaster::new(
                    Collider::rectangle(14., settings.ground_detector_height),
                    // Put detector at the feet
                    Vec2 {
                        x: 0.0,
                        y: settings.ground_detector_anchor,
                    },
                    0.0,
                    Dir2::NEG_Y,
                ),
                SpatialQueryFilter::from_mask(GameLayers::Environment),
            )
            .with_max_distance(settings.ground_detector_range),
            animation: AnimatedSpriteBundle::new(
                settings.sprite_height_offset,
                settings.animation_default_frames,
            ),
            sprite_height: EnemySpriteHeight(settings.sprite_height),
            ..EnemyCoreBundle::default()
        }
    }
}

impl Default for EnemyCoreBundle {
    fn default() -> Self {
        Self {
            enemy: Enemy,
            state: MovementState::Idle,
            body: RigidBody::Dynamic,
            friction: Friction::ZERO
                .with_combine_rule(avian2d::dynamics::rigid_body::CoefficientCombine::Min),
            speed: MovementSpeed(25.0),
            intended_x_vel: IntendedXVelocity(0.0),
            ground_detection: GroundDetection,
            ground_detector: ShapeCaster::new(
                Collider::rectangle(14., ENEMY_FOOT_HEIGHT),
                // Put detector at the feet
                Vec2 {
                    x: 0.0,
                    y: ENEMY_FOOT_ANCHOR,
                },
                0.0,
                Dir2::NEG_Y,
            )
            .with_max_distance(ENEMY_FOOT_RANGE),
            axes: LockedAxes::ROTATION_LOCKED,
            facing: FacingDirection::Right,
            sprite_height: EnemySpriteHeight::default(),
            animation: AnimatedSpriteBundle::new(0., 1),
        }
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, set_chase_target);
        app.add_plugins(orc::plugin);
        app.add_observer(on_enemy_spawned);
    }
}

fn on_enemy_spawned(
    event: On<Add, Enemy>,
    hurters: Query<&HurtsWhenTouched>,
    heights: Query<&EnemySpriteHeight>,
    mut commands: Commands,
) {
    if let Ok(height) = heights.get(event.entity) {
        commands.entity(event.entity).with_children(|parent| {
            parent.spawn((EnvColliderBundle::new(
                GameLayers::Enemies,
                [
                    GameLayers::Environment,
                    GameLayers::Props,
                    GameLayers::Enemies,
                    GameLayers::Player,
                ],
                15.5,
                height.0,
            ),));
        });
    }
    if let Ok(hurts) = hurters.get(event.entity) {
        // Add hit box in a child (which we cannot do during init because ldtk plugin does not support it)
        commands.entity(event.entity).with_children(|parent| {
            parent.spawn(HitBoxBundle::new(
                GameLayers::EnemyHitBox,
                GameLayers::PlayerHurtBox,
                hurts.width,
                hurts.height,
            ));
        });
    }
}

fn set_chase_target(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    query: Query<Entity, (With<Enemy>, Without<ChaseTarget>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(ChaseTarget(*player));
    }
}
