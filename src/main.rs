#![allow(clippy::type_complexity)]
mod ai;
mod animation;
mod attack;
mod block;
mod debug;
mod enemies;
mod movement;
mod player;
mod powers;
mod wall;

use animation::AnimationPlugin;
use avian2d::{
    PhysicsPlugins,
    debug_render::{ContactGizmoScale, PhysicsDebugPlugin, PhysicsGizmos},
    dynamics::integrator::Gravity,
};
use bevy::prelude::*;
use bevy_behave::prelude::BehavePlugin;
use bevy_ecs_ldtk::{LdtkPlugin, LdtkWorldBundle, LevelSelection};
use debug::DebugPlugin;
use enemies::EnemyPlugin;
use movement::MovementPlugin;
use player::PlayerPlugin;
use powers::powers_plugin;
use wall::WallPlugin;

use crate::{block::BlockPlugin, player::Player};

#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameSet {
    Input,
    PostInput,
    Powers,
    Animate,
    Reactions,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_world, setup_camera));
        app.add_systems(
            PostUpdate,
            follow_camera.before(TransformSystems::Propagate),
        );
        app.add_plugins((
            LdtkPlugin,
            BehavePlugin::default(),
            MovementPlugin,
            AnimationPlugin,
            PlayerPlugin,
            BlockPlugin,
            EnemyPlugin,
            WallPlugin,
            PhysicsPlugins::default().with_length_unit(50.0),
            DebugPlugin,
            ai::plugin,
            powers_plugin,
        ));
        app.insert_gizmo_config(
            PhysicsGizmos {
                contact_normal_scale: ContactGizmoScale::Constant(8.0),
                contact_normal_color: Some(Color::srgb(0.0, 1.1, 0.1)),
                contact_point_color: Some(Color::srgb(0.0, 1.0, 1.0)),
                axis_lengths: None,
                aabb_color: None,
                collider_tree_color: None,
                island_color: None,
                raycast_color: None,
                shapecast_shape_color: None,
                shapecast_normal_color: None,
                shapecast_point_color: None,
                collider_color: Some(Color::srgb(0.4, 0.6, 1.0)),
                shapecast_color: None,
                sleeping_color_multiplier: Some([1.0, 1.0, 0.4, 1.0]),
                ..default()
            },
            GizmoConfig {
                enabled: false,
                ..default()
            },
        );
        app.insert_resource(Gravity(Vec2::NEG_Y * 1000.0));
        app.insert_resource(LevelSelection::index(0));
        app.configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::PostInput,
                GameSet::Powers,
                GameSet::Animate,
                GameSet::Reactions,
            )
                .chain(),
        );
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(304.0, 232.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::AutoMin {
                min_width: 320.0,
                min_height: 240.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn follow_camera(
    players: Query<&Transform, With<Player>>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    time: Res<Time>,
) {
    for player in players.iter() {
        for mut camera in cameras.iter_mut() {
            // Keep the camera's own z coord.
            let target = Vec3::new(
                player.translation.x,
                player.translation.y,
                camera.translation.z,
            );
            camera
                .translation
                .smooth_nudge(&target, 13.9, time.delta_secs());
        }
    }
}

fn setup_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("test_map.ldtk").into(),
        ..default()
    });
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            GamePlugin,
            PhysicsDebugPlugin,
        ))
        .run();
}
