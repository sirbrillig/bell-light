use crate::{
    GameSet,
    animation::{AnimationKey, AnimationSet, CharacterAnimationClip, SpriteAnimation},
    attack::{HitBoxBundle, HurtBox},
    enemies::EnemyHurtBox,
    movement::{GameLayers, Knockback},
    player::Player,
};
use avian2d::{
    collision::collider::{CollidingEntities, collider_hierarchy::ColliderOf},
    dynamics::rigid_body::LinearVelocity,
};
use bevy::prelude::*;
use std::collections::HashMap;

const KNOCKBACK_SPEED_X: f32 = 290.0;
const KNOCKBACK_SPEED_Y: f32 = 110.0;

pub fn powers_plugin(app: &mut App) {
    app.add_systems(Startup, setup_powers);
    app.add_systems(
        Update,
        (activate_bell, process_bell)
            .chain()
            .in_set(GameSet::Powers),
    );
    app.add_systems(
        Update,
        (detect_hit, handle_got_hit, handle_directional_bell)
            .chain()
            .in_set(GameSet::Reactions),
    );
}

#[derive(Component)]
pub struct ActivateBell {
    pub direction: BellDirection,
}

impl ActivateBell {
    pub fn default() -> Self {
        Self {
            direction: BellDirection { direction: None },
        }
    }

    pub fn direction(direction: Vec2) -> Self {
        Self {
            direction: BellDirection {
                direction: Some(direction),
            },
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct BellDirection {
    pub direction: Option<Vec2>,
}

#[derive(Component)]
pub struct BellTimer {
    timer: Timer,
}

#[derive(Component)]
pub struct RepulsionBell;

#[derive(Bundle)]
struct BellBundle {
    animation_key: AnimationKey,
    sprite_sheet: Sprite,
    animation: SpriteAnimation,
    timer: BellTimer,
}

#[derive(Resource)]
pub struct PowerAnimations(AnimationSet);

fn setup_powers(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
) {
    let sprite_size = 64;
    let row_number = 5;
    let repulsion = CharacterAnimationClip {
        image: asset_server.load("explosions.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(sprite_size),
            12,
            1,
            None,
            Some(UVec2::new(0, sprite_size * row_number)),
        )),
        frames: 10,
    };
    commands.insert_resource(PowerAnimations(AnimationSet {
        animation_map: HashMap::from([(AnimationKey::Repulsion, repulsion)]),
    }));
}

fn activate_bell(
    query: Query<(Entity, &ActivateBell), Added<ActivateBell>>,
    animations: Res<PowerAnimations>,
    mut commands: Commands,
) {
    for (player, activate) in query.iter() {
        commands.entity(player).remove::<ActivateBell>();
        let clip = animations.0.clone();
        let frames = 10;
        let power_time = 0.2;
        let direction = activate.direction;
        commands.spawn((
            RepulsionBell,
            ChildOf(player),
            direction,
            clip,
            HitBoxBundle::new(
                GameLayers::PlayerPowerBox,
                [GameLayers::EnemyHurtBox, GameLayers::Props],
                45.,
                45.,
            ),
            BellBundle {
                // The timer for the power itself
                timer: BellTimer {
                    timer: Timer::from_seconds(power_time, TimerMode::Once),
                },
                animation_key: AnimationKey::Repulsion,
                sprite_sheet: Sprite::default(),
                // The timer for the animation
                animation: SpriteAnimation {
                    frames,
                    timer: Timer::from_seconds(power_time / (frames as f32), TimerMode::Repeating),
                },
            },
        ));
    }
}

fn process_bell(
    mut query: Query<(Entity, &mut BellTimer)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (bell, mut active) in query.iter_mut() {
        active.timer.tick(time.delta());
        if active.timer.is_finished() {
            commands.entity(bell).despawn();
        }
    }
}

#[derive(Component)]
pub struct HitByBell;

fn detect_hit(
    query: Query<(&CollidingEntities, &ColliderOf), (With<HurtBox>, With<EnemyHurtBox>)>,
    bells: Query<&BellDirection, With<RepulsionBell>>,
    mut commands: Commands,
) {
    for (hurtbox, owner) in query.iter() {
        let enemy = owner.body;
        for hitbox in hurtbox.iter() {
            let Ok(direction) = bells.get(*hitbox) else {
                continue;
            };
            if direction.direction.is_none() {
                commands.entity(enemy).insert(HitByBell);
            }
        }
    }
}

fn handle_directional_bell(
    bells: Query<&BellDirection, (With<RepulsionBell>, Added<RepulsionBell>)>,
    player: Single<Entity, With<Player>>,
    mut commands: Commands,
) {
    let bell_force = 300.0;
    for direction in bells.iter() {
        let Some(dir) = direction.direction else {
            continue;
        };
        commands.entity(*player).insert(Knockback {
            timer: Timer::from_seconds(0.1, TimerMode::Once),
            direction: -dir * bell_force,
        });
    }
}

fn handle_got_hit(
    query: Query<(Entity, &Transform), Added<HitByBell>>,
    colliders: Query<&Transform, With<LinearVelocity>>,
    player: Single<Entity, With<Player>>,
    mut commands: Commands,
) {
    for (enemy, target_transform) in query.iter() {
        commands.entity(enemy).remove::<HitByBell>();
        let Ok(source_transform) = colliders.get(*player) else {
            continue;
        };
        // Calculate horizontal sign (-1.0 for Left, 1.0 for Right)
        let direction_x =
            (target_transform.translation.x - source_transform.translation.x).signum();
        let direction = Vec2 {
            x: direction_x * KNOCKBACK_SPEED_X,
            y: KNOCKBACK_SPEED_Y, // Small upward pop
        };
        commands.entity(enemy).insert(Knockback {
            timer: Timer::from_seconds(0.1, TimerMode::Once),
            direction,
        });
    }
}
