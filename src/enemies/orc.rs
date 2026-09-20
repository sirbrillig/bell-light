use crate::ai::tasks::attack::{Attack, HitBoxConfig};
use crate::ai::tasks::face_target::FaceTarget;
use crate::ai::tasks::is_facing_target::IsFacingTarget;
use crate::ai::tasks::move_toward_entity::MoveTowardEntity;
use crate::ai::tasks::target_in_range::TargetInRange;
use crate::ai::tasks::wait_until_player_is_near::{DetectionDistance, WaitUntilPlayerIsNear};
use crate::animation::{AnimationClipSpec, AnimationKey, AnimationSet};
use crate::attack::HurtBoxBundle;
use crate::enemies::{EnemyCoreBundle, EnemyHurtBox, EnemySettings, HurtsWhenTouched};
use crate::movement::GameLayers;
use bevy::prelude::*;
use bevy_behave::behave;
use bevy_behave::prelude::*;
use bevy_ecs_ldtk::{LdtkEntity, app::LdtkEntityAppExt};

const ENEMY_HEIGHT: f32 = 16.0;
const ENEMY_HEIGHT_ANCHOR_OFFSET: f32 = 0.01;
const ENEMY_FOOT_HEIGHT: f32 = 2.0;
const ENEMY_FOOT_ANCHOR: f32 = -(ENEMY_HEIGHT / 2.) + (ENEMY_FOOT_HEIGHT / 2.);
const ENEMY_FOOT_RANGE: f32 = 2.0;
const CHASE_RANGE: f32 = 5000.0;

#[derive(Component, Default)]
pub struct Orc;

#[derive(Bundle, LdtkEntity)]
struct OrcBundle {
    name: Orc,
    core: EnemyCoreBundle,
    detection_distance: DetectionDistance,
    hurts: HurtsWhenTouched,
}

impl Default for OrcBundle {
    fn default() -> Self {
        Self {
            name: Orc,
            detection_distance: DetectionDistance(CHASE_RANGE),
            core: EnemyCoreBundle::with_settings(EnemySettings {
                sprite_height: ENEMY_HEIGHT,
                sprite_height_offset: ENEMY_HEIGHT_ANCHOR_OFFSET,
                speed: 25.0,
                ground_detector_height: ENEMY_FOOT_HEIGHT,
                ground_detector_anchor: ENEMY_FOOT_ANCHOR,
                ground_detector_range: ENEMY_FOOT_RANGE,
                animation_default_frames: 6,
            }),
            hurts: HurtsWhenTouched {
                width: 10.0,
                height: 10.0,
            },
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup_enemy);
    app.register_ldtk_entity::<OrcBundle>("Orc");
    app.add_observer(on_spawned);
}

#[derive(Resource)]
struct OrcAnimations(AnimationSet);

fn on_spawned(event: On<Add, Orc>, mut commands: Commands, animations: Res<OrcAnimations>) {
    commands.entity(event.entity).insert(animations.0.clone());

    // Add hurt box in a child (which we cannot do during init because ldtk plugin does not support it)
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

    let attack_range = 700.0;
    let attack = Attack {
        duration_secs: 1.5,
        active_frames: (3..5).into(),
        hitbox: HitBoxConfig {
            width: 16.0,
            height: 22.0,
            offset_x: 18.0,
            offset_y: 0.0,
        },
    };

    let tree = behave! {
        Behave::Forever => {
            Behave::Fallback => {
                Behave::Sequence => {
                   Behave::spawn_named("Is player in attack range", TargetInRange {range: attack_range}),
                   Behave::spawn_named("Is facing player", IsFacingTarget),
                   Behave::spawn_named("Attack", attack),
                },
                Behave::Sequence => {
                    Behave::spawn_named("Is player in chase range", TargetInRange {range: CHASE_RANGE}),
                    Behave::spawn_named("Face player", FaceTarget),
                    Behave::spawn_named("Move toward player", MoveTowardEntity {near_distance: attack_range, far_distance: CHASE_RANGE}),
                },
                Behave::spawn_named("Is player in at least chase range", WaitUntilPlayerIsNear),
            }
        }
    };
    commands.spawn((
        Name::new("Orc"),
        BehaveTree::new(tree),
        ChildOf(event.entity),
    ));
}

fn setup_enemy(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
) {
    let specs: &[AnimationClipSpec] = &[
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Idle,
            path: "sprites/Orc-Idle.png",
            columns: 6,
            frames: 6,
        },
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Walking,
            path: "sprites/Orc-Walk.png",
            columns: 8,
            frames: 8,
        },
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Jumping,
            path: "sprites/Orc-Walk.png",
            columns: 8,
            frames: 1,
        },
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Attacking,
            path: "sprites/Orc-Attack01.png",
            columns: 6,
            frames: 6,
        },
    ];
    commands.insert_resource(OrcAnimations(AnimationSet::from_specs(
        specs,
        &asset_server,
        &mut layouts,
    )));
}
