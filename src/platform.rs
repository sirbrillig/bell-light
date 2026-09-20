use crate::{
    attack::{HurtBox, HurtBoxBundle},
    movement::*,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

const PLATFORM_SIZE: Vec2 = Vec2::new(16., 16.);
const PLATFORM_SPEED: f32 = 30.0;

#[derive(Component, Default)]
pub struct Platform;

#[derive(Component, Default)]
pub struct PlatformHurtBox;

#[derive(Default, PartialEq)]
pub enum PlatformDirection {
    #[default]
    Forward,
    Backward,
}

#[derive(Component, Default)]
pub struct PlatformPathState {
    pub active: bool,
    pub index: usize,
    pub direction: PlatformDirection,
}

impl PlatformPathState {
    pub fn go_to_next(&mut self, max: usize) {
        if self.direction == PlatformDirection::Forward && (self.index + 1) >= max {
            self.direction = PlatformDirection::Backward;
        }
        if self.direction == PlatformDirection::Backward && self.index == 0 {
            self.direction = PlatformDirection::Forward;
        }

        if self.direction == PlatformDirection::Forward {
            self.index += 1;
        } else {
            self.index -= 1;
        }
    }
}

#[derive(Component, Default)]
pub struct PlatformPathOffsets {
    pub offsets: Vec<Vec2>,
}

#[derive(Component, Default)]
pub struct PlatformPath {
    pub points: Vec<Vec2>,
}

#[derive(Component, Default)]
pub struct BellActivate(pub bool);

#[derive(Bundle, LdtkEntity)]
pub struct PlatformBundle {
    platform: Platform,
    sprite: Sprite,
    body: RigidBody,
    axes: LockedAxes,
    #[with(create_path)]
    path: PlatformPathOffsets,
    state: PlatformPathState,
    #[with(set_bell_activate)]
    bell_activate: BellActivate,
}

fn set_bell_activate(ld_entity: &EntityInstance) -> BellActivate {
    let val = ld_entity.get_bool_field("BellActivate").unwrap_or(&false);
    BellActivate(*val)
}

fn create_path(ld_entity: &EntityInstance) -> PlatformPathOffsets {
    let platform_coord = ld_entity.grid;
    if let Ok(points_iter) = ld_entity.iter_points_field("Path") {
        let coords: Vec<IVec2> = points_iter.copied().collect();
        // coords are map grid cells originating in the upper-left (y points down) but bevy needs
        // offsets (originating at center, y points up) so we need to determine offset based on
        // ld_entity.grid which is the map cell of the platform (times platform size).
        let offsets: Vec<Vec2> = coords
            .iter()
            .map(|v| {
                let delta = v - platform_coord;
                Vec2 {
                    x: delta.x as f32 * PLATFORM_SIZE.x,
                    y: -delta.y as f32 * PLATFORM_SIZE.y,
                }
            })
            .collect();
        // We need to have the first offset be 0,0 to keep the start point in the path.
        let mut new_offsets = Vec::with_capacity(offsets.len() + 1);
        new_offsets.push(Vec2::new(0., 0.));
        new_offsets.extend(offsets);
        return PlatformPathOffsets {
            offsets: new_offsets,
        };
    }
    PlatformPathOffsets { offsets: vec![] }
}

impl Default for PlatformBundle {
    fn default() -> Self {
        Self {
            platform: Platform,
            sprite: Sprite::from_color(Color::BLACK, PLATFORM_SIZE),
            body: RigidBody::Kinematic,
            axes: LockedAxes::ROTATION_LOCKED,
            path: PlatformPathOffsets::default(),
            state: PlatformPathState::default(),
            bell_activate: BellActivate(false),
        }
    }
}

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (prepare_paths, respond_to_bell, activate_paths).chain(),
        );
        app.add_systems(FixedUpdate, move_platform);
        app.add_observer(on_spawned);
        app.register_ldtk_entity::<PlatformBundle>("Platform");
    }
}

fn respond_to_bell(
    hits: Query<(&CollidingEntities, &ColliderOf), (With<HurtBox>, With<PlatformHurtBox>)>,
    mut platforms: Query<(&BellActivate, &mut PlatformPathState)>,
) {
    for (hurtbox, owner) in hits.iter() {
        if hurtbox.is_empty() {
            continue;
        }
        let platform_entity = owner.body;
        let Ok((bell_activate, mut state)) = platforms.get_mut(platform_entity) else {
            continue;
        };
        if bell_activate.0 {
            state.active = true;
        }
    }
}

fn prepare_paths(
    query: Query<(Entity, &PlatformPathOffsets, &Transform), Without<PlatformPath>>,
    mut commands: Commands,
) {
    for (entity, offsets, transform) in query.iter() {
        let points: Vec<Vec2> = offsets
            .offsets
            .iter()
            .map(|o| transform.translation.truncate() + o)
            .collect();
        commands.entity(entity).insert(PlatformPath { points });
    }
}

fn activate_paths(mut query: Query<(&mut PlatformPathState, &BellActivate)>) {
    for (mut state, bell_activate) in query.iter_mut() {
        if !state.active && !bell_activate.0 {
            state.active = true;
        }
    }
}

fn on_spawned(event: On<Add, Platform>, mut commands: Commands) {
    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            PlatformHurtBox,
            Transform::from_xyz(0., 0.05, 0.),
            HurtBoxBundle::new(
                GameLayers::Platforms,
                GameLayers::PlayerPowerBox,
                PLATFORM_SIZE.x,
                PLATFORM_SIZE.y,
            ),
        ));
    });

    commands.entity(event.entity).with_children(|parent| {
        parent.spawn((
            Transform::from_xyz(0., 0.05, 0.),
            EnvColliderBundle::new(
                GameLayers::Platforms,
                [
                    GameLayers::Environment,
                    GameLayers::Enemies,
                    GameLayers::Player,
                ],
                PLATFORM_SIZE.x,
                PLATFORM_SIZE.y,
            ),
        ));
    });
}

fn has_arrived(delta: &Vec2, dt: f32) -> bool {
    let next_move = PLATFORM_SPEED * dt;
    delta.length() <= next_move
}

fn move_platform(
    mut query: Query<
        (
            &PlatformPath,
            &Transform,
            &mut PlatformPathState,
            &mut LinearVelocity,
        ),
        With<Platform>,
    >,
    time: Res<Time>,
) {
    for (path, transform, mut state, mut vel) in query.iter_mut() {
        if !state.active {
            continue;
        }
        let Some(point) = path.points.get(state.index) else {
            println!(
                "no next point for index {} with points {:?}",
                state.index, path.points
            );
            continue;
        };

        let delta = point - transform.translation.truncate();
        let dt = time.delta_secs();
        if has_arrived(&delta, dt) {
            state.go_to_next(path.points.len());
            // move directly to end position
            vel.0 = delta / dt;
            continue;
        }

        vel.0 = delta.normalize() * PLATFORM_SPEED;
    }
}
