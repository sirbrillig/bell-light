use crate::{
    GameSet,
    attack::Attacking,
    movement::{FacingDirection, Knockback, MovementState},
};
use avian2d::dynamics::rigid_body::LinearVelocity;
use bevy::{prelude::*, sprite::Anchor};
use std::collections::HashMap;

#[derive(Bundle)]
pub struct AnimatedSpriteBundle {
    sprite_sheet: Sprite,
    animation: SpriteAnimation,
    animation_key: AnimationKey,
    anchor: Anchor,
}

impl AnimatedSpriteBundle {
    pub fn new(anchor_offset_y: f32, animation_frames: usize) -> Self {
        Self {
            sprite_sheet: Sprite::default(),
            animation_key: AnimationKey::Idle,
            anchor: Anchor(Vec2::new(0.0, anchor_offset_y)),
            animation: SpriteAnimation {
                frames: animation_frames,
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            },
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct AnimationProgress(pub f32);

#[derive(Component, Clone)]
pub struct AnimationSet {
    pub animation_map: HashMap<AnimationKey, CharacterAnimationClip>,
}

impl AnimationSet {
    pub fn clip_for_key(&self, key: &AnimationKey) -> Option<&CharacterAnimationClip> {
        self.animation_map.get(key)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Default, Hash)]
pub enum AnimationKey {
    #[default]
    Idle,
    Walking,
    Jumping,
    Attacking,
    Repulsion,
}

#[derive(Component)]
pub struct SpriteAnimation {
    pub frames: usize,
    pub timer: Timer,
}

#[derive(Clone)]
pub struct CharacterAnimationClip {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub frames: usize,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                determine_animation_key,
                determine_facing,
                update_facing,
                update_sprites,
                animate_sprites,
            )
                .chain()
                .in_set(GameSet::Animate),
        );
    }
}

fn determine_animation_key(mut query: Query<(&MovementState, &mut AnimationKey, Has<Attacking>)>) {
    for (state, mut key, is_attacking) in query.iter_mut() {
        let next_key = match (is_attacking, state) {
            (true, _) => AnimationKey::Attacking,
            (false, MovementState::Jumping) => AnimationKey::Jumping,
            (false, MovementState::Walking) => AnimationKey::Walking,
            (false, MovementState::Idle) => AnimationKey::Idle,
        };
        if *key != next_key {
            *key = next_key;
        }
    }
}

fn animate_sprites(
    time: Res<Time>,
    mut query: Query<(
        &mut SpriteAnimation,
        &mut Sprite,
        Option<&AnimationProgress>,
    )>,
) {
    for (mut config, mut sprite, progress) in &mut query {
        config.timer.tick(time.delta());
        if config.timer.just_finished()
            && let Some(atlas) = &mut sprite.texture_atlas
        {
            atlas.index = match progress {
                Some(progress) => ((progress.0 * config.frames as f32) as usize)
                    .min(config.frames.saturating_sub(1)),
                _ => (atlas.index + 1) % config.frames.max(1),
            };
        }
    }
}

fn update_sprites(
    mut query: Query<
        (
            &AnimationKey,
            &mut Sprite,
            &mut SpriteAnimation,
            &AnimationSet,
        ),
        Changed<AnimationKey>,
    >,
) {
    for (key, mut sprite, mut animation, animation_set) in &mut query {
        let Some(clip) = animation_set.clip_for_key(key) else {
            println!("no clip for key {:?}", key);
            continue;
        };
        sprite.image = clip.image.clone();
        sprite.texture_atlas = Some(TextureAtlas {
            layout: clip.layout.clone(),
            index: 0,
        });
        animation.frames = clip.frames;
        animation.timer.reset();
    }
}

fn determine_facing(
    mut query: Query<(&mut FacingDirection, &LinearVelocity, Has<Knockback>), With<Sprite>>,
) {
    for (mut facing, vel, has_knockback) in query.iter_mut() {
        if has_knockback {
            continue;
        }
        let is_walking = vel.x.abs() > 0.1;
        if !is_walking {
            continue;
        }
        let next_facing = if vel.x > 0.0 {
            FacingDirection::Right
        } else {
            FacingDirection::Left
        };
        if *facing != next_facing {
            *facing = next_facing;
        }
    }
}

fn update_facing(mut query: Query<(&FacingDirection, &mut Sprite), Changed<FacingDirection>>) {
    for (facing, mut sprite) in &mut query {
        sprite.flip_x = *facing == FacingDirection::Left;
    }
}
