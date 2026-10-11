use crate::{
    GameSet,
    attack::Attacking,
    enemies::EnemyDying,
    movement::{FacingDirection, Knockback, MovementState},
};
use avian2d::dynamics::rigid_body::LinearVelocity;
use bevy::{prelude::*, sprite::Anchor};
use std::collections::HashMap;

// @todo A simpler approach: make the animation responsible for its own lifetime
// 1. Set the sprite up at spawn time. You already have the image handle and layout in the clip, so you don't need the key-lookup step. Build the Sprite with the image and a TextureAtlas at index 0 directly in the spawn call (Sprite::from_atlas_image does this). Then no AnimationKey or AnimationSet is involved, and update_sprites won't touch the entity because its query requires both.
//
// 2. Give SpriteAnimation a playback mode. Add a field like mode: Loop | Once (a small enum, or Once with an option to despawn when done). In animate_sprites, when the mode is Once and the index is already on the last frame, stop advancing instead of wrapping with %. That fixes the repeating explosion.
//
// 3. Despawn when the animation finishes. Either:
// - Have animate_sprites despawn the entity (via Commands) when a Once animation finishes, or
// - Insert a marker like AnimationFinished and let a separate small system despawn anything with it. This is more Bevy-like: animation code doesn't need to know about despawning, and other systems can also react to "finished" (e.g. a power that chains into something else).
//
// After that you can delete Explosion, process_dying, and most likely BellTimer/process_bell. One check on the bell: its hitbox lifetime currently equals the animation length (ANIMATION_SECS). If you ever want the hitbox to last a different time than the visual, keep BellTimer for gameplay and let the animation handle only the visuals. Those really are separate concerns.
//
// 4. Replace the one-entry maps with plain struct fields. Instead of one resource per effect, each wrapping a HashMap, use one resource like EffectClips with named fields (explosion, repulsion, …), each a CharacterAnimationClip. Field access is checked by the compiler, so there's no Option from .get() and no "no clip for key" case at runtime. It's the Rust equivalent of a typed object instead of a string-keyed map in TypeScript.
//
// 5. Optional: a helper so spawning is one line. Once the steps above are in place, a method like clip.one_shot_bundle() that returns (Sprite, SpriteAnimation) lets spawning an effect be commands.spawn((clips.explosion.one_shot_bundle(), Transform::…)). That's the payoff: adding a new effect means one new clip field and a spawn call.

/// A convenience bundle for a sprite which has a set of animations it may change bettween using the
/// AnimationKey as the current animation.
#[derive(Bundle)]
pub struct AnimatedSpriteBundle {
    sprite_sheet: Sprite,
    animation: SpriteAnimation,
    animation_key: AnimationKey,
    anchor: Anchor,
}

impl AnimatedSpriteBundle {
    pub fn new(anchor_offset_y: f32, animation_frames: usize, animation_secs: f32) -> Self {
        Self {
            sprite_sheet: Sprite::default(),
            animation_key: AnimationKey::Idle,
            anchor: Anchor(Vec2::new(0.0, anchor_offset_y)),
            animation: SpriteAnimation {
                frames: animation_frames,
                timer: Timer::from_seconds(animation_secs, TimerMode::Repeating),
            },
        }
    }
}

/// Add to an animating entity to manually control its animation. Otherwise the animation will run
/// itself based on the properties in SpriteAnimation/CharacterAnimationClip. Can be useful for
/// one-time animations like an attack sequence which need to be synchronized to a different timer.
#[derive(Component, Clone, Copy)]
pub struct AnimationProgress(pub f32);

/// A set of CharacterAnimationClips keyed by AnimationKey. Useful for sprites which have a number
/// of animations they need to switch between. Can be created using a set of AnimationClipSpecs or
/// manually. An entity with an AnimatedSpriteBundle contains a key that will be used to select and
/// play the current animation for that sprite.
#[derive(Component, Clone)]
pub struct AnimationSet {
    pub animation_map: HashMap<AnimationKey, CharacterAnimationClip>,
}

impl AnimationSet {
    pub fn clip_for_key(&self, key: &AnimationKey) -> Option<&CharacterAnimationClip> {
        self.animation_map.get(key)
    }

    /// Create an AnimationSet from a collection of AnimationClipSpecs as a convenience.
    pub fn from_specs(
        specs: &[AnimationClipSpec],
        asset_server: &AssetServer,
        layouts: &mut Assets<TextureAtlasLayout>,
    ) -> Self {
        Self {
            animation_map: specs
                .iter()
                .map(|spec| {
                    // create (key, clip) tuple pairs
                    (
                        spec.key,
                        CharacterAnimationClip {
                            image: asset_server.load(spec.path),
                            layout: layouts.add(TextureAtlasLayout::from_grid(
                                UVec2::splat(spec.tile_size),
                                spec.columns,
                                spec.rows,
                                None,
                                None,
                            )),
                            frames: spec.frames as usize,
                            timer: spec.timer.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// A helper for creating an AnimationSet. Use when there's a number of animations that a sprite
/// will change between. Create an AnimationClipSpec for each one and then use them to create an
/// AnimationSet which will be animated by the components of an AnimatedSpriteBundle.
pub struct AnimationClipSpec {
    pub tile_size: u32,
    pub key: AnimationKey,
    pub path: &'static str,
    pub columns: u32,
    pub rows: u32,
    pub frames: u32,
    pub timer: Timer,
}

/// The key for an AnimationSet which is usually specified by the key on an AnimatedSpriteBundle.
/// These are the common states for all sprites using the animation system.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Default, Hash)]
pub enum AnimationKey {
    #[default]
    Idle,
    Walking,
    Jumping,
    Attacking,
    Repulsion,
    Shatter,
    Explode,
}

/// A timer to run an animation.
#[derive(Component)]
pub struct SpriteAnimation {
    pub frames: usize,
    pub timer: Timer,
}

/// A sprite animation which will be cloned and used when animating. Its timer will never change; it
/// is used to set the SpriteAnimation timer that runs the animation itself.
#[derive(Clone)]
pub struct CharacterAnimationClip {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub frames: usize,
    pub timer: Timer,
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

fn get_next_key(
    is_dying: bool,
    is_attacking: bool,
    movement_state: &MovementState,
) -> AnimationKey {
    if is_dying {
        return AnimationKey::Shatter;
    }
    if is_attacking {
        return AnimationKey::Attacking;
    }
    match movement_state {
        MovementState::Jumping => AnimationKey::Jumping,
        MovementState::Walking => AnimationKey::Walking,
        MovementState::Idle => AnimationKey::Idle,
    }
}

/// Possibly update the AnimationKey of an AnimatedSpriteBundle if its other components need that to
/// happen (see get_next_key). Once the key updates, update_sprites will swap out the
/// SpriteAnimation for the one matching the new key and start playing it.
fn determine_animation_key(
    mut query: Query<(
        &MovementState,
        &mut AnimationKey,
        Has<Attacking>,
        Has<EnemyDying>,
    )>,
) {
    for (movement_state, mut key, is_attacking, is_dying) in query.iter_mut() {
        let next_key = get_next_key(is_dying, is_attacking, movement_state);
        if *key != next_key {
            *key = next_key;
        }
    }
}

/// Run the SpriteAnimation by moving to the next frame. SpriteAnimation has its own timer that
/// controls this but it can be controlled manually if an entity has AnimationProgress which can be
/// useful to sync an animation to some other timer (like an attack).
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

// Any time the AnimationKey changes, get the appropriate new CharacterAnimationClip out of the
// AnimationSet and use it to replace the current SpriteAnimation settings.
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
        animation.timer = clip.timer.clone();
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
