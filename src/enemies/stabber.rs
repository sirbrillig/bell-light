use crate::ai::tasks::charge_straight::{ChargeDirection, ChargeStraight};
use crate::ai::tasks::wait_until_player_is_near::{DetectionDistance, WaitUntilPlayerIsNear};
use crate::animation::{AnimationClipSpec, AnimationKey, AnimationSet};
use crate::attack::HurtBoxBundle;
use crate::enemies::{EnemyCoreBundle, EnemyHurtBox, EnemySettings, HurtsWhenTouched};
use crate::movement::{GameLayers, OrthagonalDirection};
use avian2d::collision::collider::collider_hierarchy::ColliderOf;
use avian2d::collision::collider::{Collider, CollidingEntities, CollisionLayers, LayerMask};
use avian2d::dynamics::rigid_body::{LinearVelocity, RigidBody};
use avian2d::math::{FRAC_PI_2, PI};
use bevy::prelude::*;
use bevy_behave::behave;
use bevy_behave::prelude::*;
use bevy_ecs_ldtk::EntityInstance;
use bevy_ecs_ldtk::ldtk::ldtk_fields::LdtkFields;
use bevy_ecs_ldtk::{LdtkEntity, app::LdtkEntityAppExt};

const ENEMY_HEIGHT: f32 = 16.0;
const ENEMY_HEIGHT_ANCHOR_OFFSET: f32 = 0.01;
const ENEMY_FOOT_HEIGHT: f32 = 2.0;
const ENEMY_FOOT_ANCHOR: f32 = -(ENEMY_HEIGHT / 2.) + (ENEMY_FOOT_HEIGHT / 2.);
const ENEMY_FOOT_RANGE: f32 = 2.0;
const ATTACK_RANGE: f32 = 800.0;
const ATTACK_SPEED: f32 = 100.0;

#[derive(Component, Default)]
pub struct Stabber;

#[derive(Bundle, LdtkEntity)]
struct StabberBundle {
    name: Stabber,
    core: EnemyCoreBundle,
    detection_distance: DetectionDistance,
    hurts: HurtsWhenTouched,
    #[with(set_charge_direction)]
    charge_direction: ChargeDirection,
}

impl Default for StabberBundle {
    fn default() -> Self {
        Self {
            name: Stabber,
            detection_distance: DetectionDistance(ATTACK_RANGE),
            core: EnemyCoreBundle::with_settings(EnemySettings {
                body_type: RigidBody::Kinematic,
                sprite_height: ENEMY_HEIGHT,
                sprite_height_offset: ENEMY_HEIGHT_ANCHOR_OFFSET,
                speed: ATTACK_SPEED,
                ground_detector_height: ENEMY_FOOT_HEIGHT,
                ground_detector_anchor: ENEMY_FOOT_ANCHOR,
                ground_detector_range: ENEMY_FOOT_RANGE,
                animation_default_frames: 1,
            }),
            hurts: HurtsWhenTouched {
                width: 10.0,
                height: 10.0,
            },
            charge_direction: ChargeDirection(OrthagonalDirection::Right),
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup_enemy);
    app.add_systems(Update, rotate_sprite);
    app.add_systems(FixedUpdate, collide_with_environment);
    app.register_ldtk_entity::<StabberBundle>("Stabber");
    app.add_observer(on_spawned);
}

#[derive(Resource)]
struct StabberAnimations(AnimationSet);

#[derive(Component, Default)]
pub struct SurfaceCollider;

#[derive(Bundle)]
pub struct SurfaceColliderBundle {
    pub name: SurfaceCollider,
    pub layers: CollisionLayers,
    pub collider: Collider,
    pub entities: CollidingEntities,
}

impl SurfaceColliderBundle {
    pub fn new(
        layer: GameLayers,
        other_layers: impl Into<LayerMask>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            name: SurfaceCollider,
            layers: CollisionLayers::new(layer, other_layers),
            collider: Collider::rectangle(width, height),
            entities: CollidingEntities::default(),
        }
    }
}

fn on_spawned(
    event: On<Add, Stabber>,
    mut query: Query<&ChargeDirection, With<Stabber>>,
    mut commands: Commands,
    animations: Res<StabberAnimations>,
) {
    commands.entity(event.entity).insert(animations.0.clone());

    // @todo allow destroying with bell rather than knockback
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            EnemyHurtBox,
            HurtBoxBundle::new(
                GameLayers::EnemyHurtBox,
                GameLayers::PlayerPowerBox,
                10.,
                10.,
            ),
        ));
    });

    let Ok(direction) = query.get_mut(event.entity) else {
        return;
    };

    // Note that this will rotate with the sprite; see rotate_sprite
    let surface_collider_offset = 6.0;
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            SurfaceColliderBundle::new(GameLayers::Enemies, GameLayers::Environment, 4., 4.),
            Transform::from_xyz(0., surface_collider_offset, 0.0),
        ));
    });

    let attack = ChargeStraight::from(direction);
    let tree = behave! {
        Behave::Forever => {
            Behave::Sequence => {
                Behave::spawn_named("Is player in attack range", WaitUntilPlayerIsNear),
                Behave::spawn_named("Attack", attack),
            },
        }
    };
    commands.spawn((
        Name::new("Stabber"),
        BehaveTree::new(tree),
        ChildOf(event.entity),
    ));
}

fn rotate_sprite(mut query: Query<(&ChargeDirection, &mut Transform), Added<Stabber>>) {
    for (direction, mut transform) in query.iter_mut() {
        // Rotate sprite to face direction; it is normally facing up
        if direction.0 == OrthagonalDirection::Down {
            transform.rotation = Quat::from_rotation_z(PI);
        }
        if direction.0 == OrthagonalDirection::Left {
            transform.rotation = Quat::from_rotation_z(FRAC_PI_2);
        }
        if direction.0 == OrthagonalDirection::Right {
            transform.rotation = Quat::from_rotation_z(-FRAC_PI_2);
        }
    }
}

fn setup_enemy(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
) {
    let specs: &[AnimationClipSpec] = &[
        AnimationClipSpec {
            tile_size: 16,
            key: AnimationKey::Idle,
            path: "sprites/bloodstoneOre.png",
            columns: 1,
            frames: 1,
        },
        AnimationClipSpec {
            tile_size: 16,
            key: AnimationKey::Jumping,
            path: "sprites/bloodstoneOre.png",
            columns: 1,
            frames: 1,
        },
        AnimationClipSpec {
            tile_size: 16,
            key: AnimationKey::Attacking,
            path: "sprites/bloodstoneOre.png",
            columns: 1,
            frames: 1,
        },
    ];
    commands.insert_resource(StabberAnimations(AnimationSet::from_specs(
        specs,
        &asset_server,
        &mut layouts,
    )));
}

fn collide_with_environment(
    movers: Query<(Entity, &LinearVelocity), With<Stabber>>,
    colliders: Query<(&CollidingEntities, &ColliderOf), With<SurfaceCollider>>,
    mut commands: Commands,
) {
    for (collider, owner) in colliders.iter() {
        if collider.is_empty() {
            continue;
        }
        let Ok((mover, vel)) = movers.get(owner.body) else {
            continue;
        };
        // If we hit a surface while moving, destroy the mover
        if vel.x != 0. || vel.y != 0. {
            // @todo add destruction animation
            commands.entity(mover).despawn();
        }
    }
}

fn set_charge_direction(ld_entity: &EntityInstance) -> ChargeDirection {
    let val = ld_entity.get_enum_field("Direction").unwrap();
    ChargeDirection(OrthagonalDirection::from(val as &str))
}
