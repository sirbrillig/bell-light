use crate::GameSet;
use crate::animation::{AnimatedSpriteBundle, AnimationClipSpec, AnimationKey, AnimationSet};
use crate::attack::{HitBox, HurtBox, HurtBoxBundle};
use crate::movement::*;
use crate::powers::ActivateBell;
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::{LdtkEntity, Worldly, app::LdtkEntityAppExt};

const PLAYER_JUMP_SPEED: f32 = 255.0;
const PLAYER_JUMP_CUT_SPEED: f32 = 190.0;
const PLAYER_HEIGHT: f32 = 20.0;
const PLAYER_WIDTH: f32 = 12.0;
const PLAYER_HEAD_CLEARANCE: f32 = 4.0;
const PLAYER_ENV_COLLIDER_HEIGHT: f32 = 14.0;
const PLAYER_SPRITE_ANCHOR_OFFSET: f32 = 0.05;
const PLAYER_FOOT_HEIGHT: f32 = 1.5;
const PLAYER_FOOT_ANCHOR: f32 = -(PLAYER_HEIGHT / 2.) + (PLAYER_FOOT_HEIGHT / 2.);
const KNOCKBACK_SPEED_X: f32 = 290.0;
const KNOCKBACK_SPEED_Y: f32 = 110.0;

#[derive(Resource)]
pub struct PlayerAnimations(AnimationSet);

#[derive(Component, Default)]
pub struct Player;

#[derive(Bundle, LdtkEntity)]
struct PlayerBundle {
    player: Player,
    state: MovementState,
    #[worldly]
    worldly: Worldly,
    body: RigidBody,
    friction: Friction,
    speed: MovementSpeed,
    // @todo add wall detection
    ground_detection: GroundDetectionBundle,
    coyote_time: CoyoteTimer,
    axes: LockedAxes,
    facing: FacingDirection,
    animation: AnimatedSpriteBundle,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        Self {
            player: Player,
            state: MovementState::Idle,
            worldly: Worldly::default(),
            body: RigidBody::Dynamic,
            friction: Friction::ZERO
                .with_combine_rule(avian2d::dynamics::rigid_body::CoefficientCombine::Min),
            speed: MovementSpeed(90.0),
            ground_detection: GroundDetectionBundle::new(
                Vec2::new(8., PLAYER_FOOT_HEIGHT),
                Vec2::new(0., PLAYER_FOOT_ANCHOR),
            ),
            coyote_time: CoyoteTimer::default(),
            axes: LockedAxes::ROTATION_LOCKED,
            animation: AnimatedSpriteBundle::new(PLAYER_SPRITE_ANCHOR_OFFSET, 6, 0.1),
            facing: FacingDirection::Right,
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, move_player.in_set(GameSet::Input));
        app.add_systems(
            Update,
            (detect_hit, handle_got_hit, handle_hurt, handle_invincible)
                .chain()
                .in_set(GameSet::Reactions),
        );
        app.register_ldtk_entity::<PlayerBundle>("Player");
        app.add_observer(on_player_spawned);
    }
}

fn on_player_spawned(
    event: On<Add, Player>,
    mut commands: Commands,
    animations: Res<PlayerAnimations>,
) {
    // Add player animation map (must do in a System so we can access World things like commands)
    commands.entity(event.entity).insert(animations.0.clone());

    // Add colliders in a child (which we cannot do during init because ldtk plugin does not support it)
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            EnvColliderBundle::new(
                GameLayers::Player,
                [
                    GameLayers::Environment,
                    GameLayers::Props,
                    GameLayers::Platforms,
                ],
                PLAYER_WIDTH,
                // Note: this must not be the same height as a tile or it will cause strange
                // ghost collisions.
                PLAYER_ENV_COLLIDER_HEIGHT,
            ),
            Transform::from_xyz(0.0, -PLAYER_HEAD_CLEARANCE, 0.0),
            // SpeculativeMargin puts a cap on avian2d's contact preditiction so that we don't hit
            // imaginary planes when jumping.
            SpeculativeMargin(1.0),
        ));
    });
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            PlayerHurtBox,
            HurtBoxBundle::new(GameLayers::PlayerHurtBox, GameLayers::EnemyHitBox, 10., 14.),
            Transform::from_xyz(0.0, -4.0, 0.0),
        ));
    });
}

#[derive(Component)]
pub struct PlayerHurtBox;

#[derive(Component)]
pub struct GotHit {
    hit_by: Entity,
}

#[derive(Component)]
pub struct HurtState {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Invincible {
    pub timer: Timer,
}

fn detect_hit(
    query: Query<&CollidingEntities, (With<HurtBox>, With<PlayerHurtBox>)>,
    player: Single<Entity, With<Player>>,
    hitbox_owners: Query<&ColliderOf, With<HitBox>>,
    mut commands: Commands,
) {
    for hurtbox in query.iter() {
        for hitbox in hurtbox.iter() {
            let Ok(owner) = hitbox_owners.get(*hitbox) else {
                continue;
            };
            let enemy = owner.body;
            commands.entity(*player).insert(GotHit { hit_by: enemy });
        }
    }
}

fn handle_got_hit(
    query: Query<(Entity, &GotHit, &Transform, Has<Invincible>), Added<GotHit>>,
    colliders: Query<&Transform, With<LinearVelocity>>,
    mut commands: Commands,
) {
    for (player, hit, player_transform, invincible) in query.iter() {
        commands.entity(player).remove::<GotHit>();
        if invincible {
            continue;
        }
        // Add HurtState to visually show the player get hurt
        commands.entity(player).insert(HurtState {
            timer: Timer::from_seconds(0.1, TimerMode::Once),
        });
        // Add Knockback to knock the player back
        let Ok(enemy_transform) = colliders.get(hit.hit_by) else {
            continue;
        };
        // Calculate horizontal sign (-1.0 for Left, 1.0 for Right)
        let direction_x = (player_transform.translation.x - enemy_transform.translation.x).signum();
        let direction = Vec2 {
            x: direction_x * KNOCKBACK_SPEED_X,
            y: KNOCKBACK_SPEED_Y, // Small upward pop
        };
        commands.entity(player).insert(Knockback {
            timer: Timer::from_seconds(0.1, TimerMode::Once),
            direction,
        });
        // Make player invincible briefly
        commands.entity(player).insert(Invincible {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
        });
    }
}

fn handle_hurt(
    mut query: Query<(&mut Sprite, &mut HurtState, Entity)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (mut _sprite, mut hurt, entity) in query.iter_mut() {
        if hurt.timer.is_finished() {
            commands.entity(entity).remove::<HurtState>();
            continue;
        }
        hurt.timer.tick(time.delta());
        // @todo play a hurt animation (time it to the hurt timer using AnimationProgress)
    }
}

fn handle_invincible(
    mut query: Query<(&mut Sprite, &mut Invincible, Entity)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (mut sprite, mut invincible, entity) in query.iter_mut() {
        if invincible.timer.is_finished() {
            sprite.color.set_alpha(1.0);
            commands.entity(entity).remove::<Invincible>();
            continue;
        }
        invincible.timer.tick(time.delta());
        let elapsed = invincible.timer.elapsed_secs();
        let flicker_hz = 15.0;
        let alpha = if ((elapsed * flicker_hz) as u32).is_multiple_of(2) {
            0.3
        } else {
            1.0
        };
        sprite.color.set_alpha(alpha);
    }
}

fn get_change_for_input(keyboard_input: &ButtonInput<KeyCode>) -> f32 {
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        1.0
    } else if keyboard_input.pressed(KeyCode::ArrowLeft) {
        -1.0
    } else {
        0.0
    }
}

fn get_input_direction(keyboard_input: &ButtonInput<KeyCode>, on_ground: bool) -> Option<Vec2> {
    if keyboard_input.pressed(KeyCode::ArrowDown) && on_ground {
        Some(Vec2::NEG_Y)
    // @todo disabled until we have wall detection
    // } else if keyboard_input.pressed(KeyCode::ArrowRight) {
    //     Some(Vec2::X)
    // } else if keyboard_input.pressed(KeyCode::ArrowLeft) {
    //     // @todo only do this if touching the wall based on the direction
    //     Some(Vec2::NEG_X)
    } else {
        None
    }
}

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player: Single<
        (
            Entity,
            &mut LinearVelocity,
            &MovementSpeed,
            &mut CoyoteTimer,
            Has<OnGround>,
        ),
        (With<Player>, Without<CannotMove>),
    >,
    mut commands: Commands,
) {
    let (entity, mut vel, speed, mut coyote, on_ground) = player.into_inner();

    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Some(direction) = get_input_direction(&keyboard_input, on_ground) {
            commands
                .entity(entity)
                .insert(ActivateBell::direction(direction));
        } else {
            commands.entity(entity).insert(ActivateBell::default());
        }
    }

    vel.x = get_change_for_input(&keyboard_input) * speed.0;
    if keyboard_input.just_released(KeyCode::ArrowUp) && vel.0.y > 0.0 {
        vel.0.y = vel.0.y.min(PLAYER_JUMP_CUT_SPEED);
    }
    if coyote.can_jump() && keyboard_input.just_pressed(KeyCode::ArrowUp) {
        vel.y = PLAYER_JUMP_SPEED;
        // End the timer when actually jumping.
        coyote.end();
    }
}

fn setup_player(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
) {
    let specs: &[AnimationClipSpec] = &[
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Idle,
            path: "Priest-Idle.png",
            columns: 6,
            frames: 6,
        },
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Walking,
            path: "Priest-Walk.png",
            columns: 8,
            frames: 8,
        },
        AnimationClipSpec {
            tile_size: 100,
            key: AnimationKey::Jumping,
            path: "Priest-Walk.png",
            columns: 8,
            frames: 1,
        },
    ];
    commands.insert_resource(PlayerAnimations(AnimationSet::from_specs(
        specs,
        &asset_server,
        &mut layouts,
    )));
}
