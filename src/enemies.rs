use std::collections::HashMap;

use crate::ai::tasks::move_toward_entity::ChaseTarget;
use crate::animation::{
    AnimatedSpriteBundle, AnimationKey, AnimationSet, CharacterAnimationClip, SpriteAnimation,
};
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
pub mod stabber;

// These are defaults, they will probably need to be overridden
const ENEMY_HEIGHT: f32 = 16.0;
const ENEMY_HEIGHT_ANCHOR_OFFSET: f32 = 0.01;
const ENEMY_FOOT_HEIGHT: f32 = 2.0;
const ENEMY_FOOT_ANCHOR: f32 = -(ENEMY_HEIGHT / 2.) + (ENEMY_FOOT_HEIGHT / 2.);
const ENEMY_FOOT_RANGE: f32 = 2.0;

const EXPLODE_SECS: f32 = 0.3;
const EXPLODE_FRAMES: usize = 9;

#[derive(Component, Default)]
pub struct Enemy;

#[derive(Component, Default)]
pub struct EnemyHurtBox;

#[derive(Component)]
pub struct EnemyDying;

#[derive(Component)]
pub struct Explosion {
    timer: Timer,
}

impl Default for Explosion {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Once),
        }
    }
}

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
    intended_vel: IntendedVelocity,
    ground_detection: GroundDetection,
    ground_detector: ShapeCaster,
    axes: LockedAxes,
    animation: AnimatedSpriteBundle,
    facing: FacingDirection,
}

#[derive(Component, Default, Clone, Copy)]
struct EnemySpriteHeight(f32);

pub struct EnemySettings {
    pub sprite_height: f32,
    pub sprite_height_offset: f32,
    pub speed: f32,
    pub ground_detector_height: f32,
    pub ground_detector_anchor: f32,
    pub ground_detector_range: f32,
    pub animation_default_frames: usize,
    pub animation_default_secs: f32,
    pub body_type: RigidBody,
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
            animation_default_secs: 0.1,
            body_type: RigidBody::Dynamic,
        }
    }
}

impl EnemyCoreBundle {
    pub fn with_settings(settings: EnemySettings) -> Self {
        Self {
            speed: MovementSpeed(settings.speed),
            body: settings.body_type,
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
            // This adds a default animation to the enemy, but that will change whenever
            // AnimationKey is changed.
            animation: AnimatedSpriteBundle::new(
                settings.sprite_height_offset,
                settings.animation_default_frames,
                settings.animation_default_secs,
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
            intended_vel: IntendedVelocity::x(0.0),
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
            animation: AnimatedSpriteBundle::new(0., 1, 0.1),
        }
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_explosion);
        app.add_systems(Update, (set_chase_target, start_dying, process_dying));
        app.add_plugins((orc::plugin, stabber::plugin));
        app.add_observer(on_enemy_spawned);
    }
}

#[derive(Bundle)]
pub struct ExplosionBundle {
    timer: Explosion,
    body: RigidBody,
    animation_key: AnimationKey,
    sprite_sheet: Sprite,
    animation: SpriteAnimation,
}

impl Default for ExplosionBundle {
    fn default() -> Self {
        Self {
            timer: Explosion::default(),
            body: RigidBody::Static,
            animation_key: AnimationKey::Explode,
            sprite_sheet: Sprite::default(),
            animation: SpriteAnimation {
                frames: EXPLODE_FRAMES,
                timer: Timer::from_seconds(
                    EXPLODE_SECS / (EXPLODE_FRAMES as f32),
                    TimerMode::Repeating,
                ),
            },
        }
    }
}

#[derive(Resource)]
pub struct ExplosionAnimations(AnimationSet);

fn setup_explosion(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
) {
    let sprite_size = 32;
    let row_number = 1;
    let clip = CharacterAnimationClip {
        image: asset_server.load("sprites/explosion-1.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(sprite_size),
            12,
            1,
            None,
            Some(UVec2::new(0, sprite_size * row_number)),
        )),
        frames: EXPLODE_FRAMES,
        // @todo the explosion repeats and it should only happen once
        timer: Timer::from_seconds(EXPLODE_SECS / (EXPLODE_FRAMES as f32), TimerMode::Repeating),
    };
    commands.insert_resource(ExplosionAnimations(AnimationSet {
        animation_map: HashMap::from([(AnimationKey::Explode, clip)]),
    }));
}

fn start_dying(
    enemies: Query<(Entity, &Transform), Added<EnemyDying>>,
    mut commands: Commands,
    animations: Res<ExplosionAnimations>,
) {
    for (enemy, pos) in enemies.iter() {
        let clip = animations.0.clone();
        commands.spawn((
            ExplosionBundle::default(),
            clip,
            // Put the explosion on the map where the enemy is
            Transform::from_xyz(pos.translation.x, pos.translation.y, 0.0),
        ));
        commands.entity(enemy).despawn();
    }
}

fn process_dying(
    mut animating: Query<(Entity, &mut Explosion)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut explosion) in animating.iter_mut() {
        explosion.timer.tick(time.delta());
        if explosion.timer.is_finished() {
            commands.entity(entity).despawn();
        }
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
