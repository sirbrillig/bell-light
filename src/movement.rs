use crate::GameSet;
use avian2d::prelude::*;
use bevy::prelude::*;

pub struct MovementPlugin;

const GROUND_DETECTION_RANGE: f32 = 2.0;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                determine_cannot_move,
                ground_detection,
                coyote_timer,
                set_intended_velocity,
            )
                .chain()
                .before(GameSet::Input),
        );
        app.add_systems(Update, determine_movement_state.in_set(GameSet::PostInput));
        app.add_systems(Update, handle_knockback.in_set(GameSet::Reactions));
    }
}

#[derive(Component, Default)]
pub struct EnvCollider;

#[derive(Bundle)]
pub struct EnvColliderBundle {
    pub env_collider: EnvCollider,
    pub layers: CollisionLayers,
    pub collider: Collider,
}

impl EnvColliderBundle {
    pub fn new(
        layer: GameLayers,
        other_layers: impl Into<LayerMask>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            env_collider: EnvCollider,
            layers: CollisionLayers::new(layer, other_layers),
            collider: Collider::rectangle(width, height),
        }
    }
}

#[derive(PhysicsLayer, Default)]
pub enum GameLayers {
    #[default]
    Environment,
    Platforms,
    Props,
    Player,
    PlayerHurtBox,
    Enemies,
    EnemyHitBox,
    EnemyHurtBox,
    PlayerPowerBox,
}

#[derive(Component)]
pub struct GroundDetection;

#[derive(Bundle)]
pub struct GroundDetectionBundle {
    detection: GroundDetection,
    ground_detector: ShapeCaster,
}

impl GroundDetectionBundle {
    pub fn new(shape: Vec2, origin: Vec2) -> Self {
        Self {
            detection: GroundDetection,
            ground_detector: ShapeCaster::with_query_filter(
                ShapeCaster::new(
                    Collider::rectangle(shape.x, shape.y),
                    origin,
                    0.0,
                    Dir2::NEG_Y,
                ),
                SpatialQueryFilter::from_mask([
                    GameLayers::Environment,
                    GameLayers::Platforms,
                    GameLayers::Props,
                ]),
            )
            .with_max_distance(GROUND_DETECTION_RANGE),
        }
    }
}

#[derive(Component)]
pub struct OnGround;

#[derive(Component, Copy, Clone)]
pub struct MovementSpeed(pub f32);

#[derive(Component, Copy, Clone)]
pub struct IntendedVelocity(pub Vec2);

impl IntendedVelocity {
    pub fn x(x_vel: f32) -> Self {
        Self(Vec2::new(x_vel, 0.))
    }
}

#[derive(Component)]
pub struct Knockback {
    pub timer: Timer,
    pub direction: Vec2,
}

#[derive(Component, Default)]
pub struct CannotMove;

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Default, Hash)]
pub enum MovementState {
    #[default]
    Idle,
    Walking,
    Jumping,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Default)]
#[allow(dead_code)]
pub enum OrthagonalDirection {
    #[default]
    Right,
    Down,
    Left,
    Up,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FacingDirection {
    #[default]
    Right,
    Left,
}

#[derive(Component)]
pub struct CoyoteTimer {
    timer: Timer,
}

impl Default for CoyoteTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.1, TimerMode::Once);
        // Start completed so it has to be started explicitly.
        timer.tick(timer.remaining());
        CoyoteTimer { timer }
    }
}

impl CoyoteTimer {
    pub fn end(&mut self) {
        self.timer.tick(self.timer.remaining());
    }

    pub fn can_jump(&self) -> bool {
        !self.timer.is_finished()
    }
}

fn handle_knockback(
    mut query: Query<(&mut LinearVelocity, &mut Knockback, Entity)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (mut vel, mut knock, entity) in query.iter_mut() {
        if knock.timer.is_finished() {
            vel.x = 0.0;
            commands.entity(entity).remove::<Knockback>();
            continue;
        }
        knock.timer.tick(time.delta());
        vel.x = knock.direction.x;
        vel.y = knock.direction.y;
    }
}

fn determine_movement_state(
    mut query: Query<(&mut MovementState, &LinearVelocity, Has<OnGround>)>,
) {
    for (mut state, vel, on_ground) in query.iter_mut() {
        let is_walking = vel.x.abs() > 0.1;
        let next_state = match (on_ground, is_walking) {
            (false, _) => MovementState::Jumping,
            (true, true) => MovementState::Walking,
            (true, false) => MovementState::Idle,
        };
        if *state != next_state {
            *state = next_state;
        }
    }
}

fn determine_cannot_move(
    mut commands: Commands,
    query: Query<
        (Entity, Has<Knockback>, Has<CannotMove>),
        Or<(With<Knockback>, With<CannotMove>)>,
    >,
) {
    for (entity, is_knockback, cannot_move) in query.iter() {
        // @note this array is here so it can be expanded later for other states
        let next_cannot_move = [is_knockback].iter().any(|&x| x);
        if next_cannot_move == cannot_move {
            continue;
        }
        if next_cannot_move {
            commands.entity(entity).insert(CannotMove);
        } else {
            commands.entity(entity).remove::<CannotMove>();
        }
    }
}

fn ground_detection(
    mut commands: Commands,
    query: Query<(Entity, &ShapeHits, Has<OnGround>), With<GroundDetection>>,
) {
    for (entity, hits, was_on_ground) in query {
        let is_on_ground = !hits.is_empty();
        if is_on_ground == was_on_ground {
            continue;
        }
        if is_on_ground {
            commands.entity(entity).insert(OnGround);
        } else {
            commands.entity(entity).remove::<OnGround>();
        }
    }
}

fn coyote_timer(
    time: Res<Time>,
    query: Query<(Has<OnGround>, &LinearVelocity, &mut CoyoteTimer), With<GroundDetection>>,
) {
    for (is_on_ground, vel, mut coyote) in query {
        // When on the ground and not moving upward (to catch frames when OnGround has not yet been
        // removed), start the timer.
        if is_on_ground && vel.y <= 0.1 {
            coyote.timer.reset();
            continue;
        }
        coyote.timer.tick(time.delta());
    }
}

fn set_intended_velocity(mut query: Query<(&mut LinearVelocity, &IntendedVelocity)>) {
    for (mut vel, intent) in query.iter_mut() {
        vel.0 = intent.0;
    }
}
