//! # Game Audio Manager
//!
//! A comprehensive audio management system for games built on top of the Kira audio library.
//! This module provides spatial audio capabilities, footstep management, enemy audio tracking,
//! background music control, and various sound effects with volume control.
//!
//! ## Features
//!
//! - **Spatial Audio**: 3D positional audio with distance-based attenuation and reverb
//! - **Movement Audio**: Footstep sounds that adapt to walking/sprinting states
//! - **Enemy Audio Management**: Individual tracking and positioning of enemy sounds
//! - **Background Music**: Looping background music with volume control for different game states
//! - **Sound Effects**: Various game sounds (completion, wall hits, UI interactions, etc.)
//! - **Volume Management**: Dynamic volume adjustment for different game contexts
//!
//! ## Usage
//!
//! ```rust
//! use your_crate::GameAudioManager;
//!
//! // Initialize the audio manager
//! let mut audio_manager = GameAudioManager::new()?;
//!
//! // Set listener position (typically the player position)
//! audio_manager.set_listener_position([0.0, 0.0, 0.0])?;
//!
//! // Start player movement audio
//! audio_manager.start_walking()?;
//!
//! // Spawn an enemy with spatial audio
//! audio_manager.spawn_enemy("enemy_1".to_string(), [10.0, 0.0, 5.0])?;
//!
//! // Update enemy position as it moves
//! audio_manager.update_enemy_position("enemy_1", [8.0, 0.0, 3.0])?;
//! ```

use crate::assets;
use kira::Decibels;
use kira::sound::static_sound::StaticSoundSettings;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Easing, Mapping, StartTime, Tween, Value,
    effect::{reverb::ReverbBuilder, volume_control::VolumeControlBuilder},
    listener::ListenerHandle,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    track::{SpatialTrackBuilder, SpatialTrackDistances, SpatialTrackHandle},
};
use std::time::Instant;

use std::{collections::HashMap, error::Error, time::Duration};

/// Represents the different movement states for footstep audio management.
///
/// Each state corresponds to different footstep timing and audio characteristics:
/// - `Idle`: No movement, no footstep sounds
/// - `Walking`: Normal paced footsteps with standard loop timing
/// - `Sprinting`: Faster footsteps with reduced loop timing for urgency
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementState {
    /// No movement - footstep audio is stopped
    Idle,
    /// Normal walking pace - footsteps loop every 0.5 seconds
    Walking,
    /// Fast movement - footsteps loop every 0.25 seconds
    Sprinting,
}

/// The main audio manager for game audio systems.
///
/// `GameAudioManager` handles all aspects of game audio including:
/// - 3D spatial audio with distance-based effects
/// - Dynamic footstep audio based on movement state
/// - Individual enemy audio tracking with spatial positioning
/// - Background music with adaptive volume control
/// - Various sound effects with cooldown management
///
/// The manager uses the Kira audio library for high-quality audio processing
/// and provides a simple interface for game developers to integrate audio.
pub struct GameAudioManager {
    /// Core Kira audio manager instance
    audio_manager: AudioManager<DefaultBackend>,

    /// Audio listener handle for spatial audio calculations
    /// The listener typically represents the player's position and orientation
    listener: ListenerHandle,

    /// Current footstep sound handle, if playing
    /// Managed automatically based on movement state
    footstep_sound: Option<StaticSoundHandle>,

    /// Map of enemy IDs to their corresponding audio handles
    /// Allows individual control of enemy audio (pause, resume, stop)
    enemy_sounds: HashMap<String, StaticSoundHandle>,

    /// Pre-loaded audio data for footstep sounds
    /// Single step audio that gets looped at different rates
    footstep_data: StaticSoundData,

    /// Pre-loaded audio data for enemy sounds
    /// Typically a looping ambient sound (e.g., slime movement)
    enemy_data: StaticSoundData,

    /// Audio data for level/objective completion sound
    complete_data: StaticSoundData,

    /// Audio data for wall collision sound effects
    wall_hit_data: StaticSoundData,

    /// Audio data for UI selection/menu sounds
    select_data: StaticSoundData,

    /// Audio data for upgrade/power-up sounds
    upgrade_data: StaticSoundData,

    /// Audio data for background music track
    background_music_data: StaticSoundData,

    /// Handle for the currently playing background music
    /// Used for volume control and stopping/starting music
    background_music_handle: Option<StaticSoundHandle>,

    /// Map of enemy IDs to their spatial audio tracks
    /// Each track handles 3D positioning, distance attenuation, and effects
    spatial_tracks: HashMap<String, SpatialTrackHandle>,

    /// Current movement state for footstep management
    movement_state: MovementState,

    /// Minimum time between wall hit sound effects
    /// Prevents audio spam when repeatedly hitting walls
    wall_hit_cooldown: Duration,

    /// Timestamp of the last wall hit sound
    /// Used with cooldown to manage sound effect timing
    last_wall_hit: Option<Instant>,

    /// Audio data for beeper rise sound effect
    /// Made public for external access if needed
    pub beeper_rise_data: StaticSoundData,
}

impl GameAudioManager {
    /// Creates a new `GameAudioManager` instance with all audio assets loaded.
    ///
    /// This constructor:
    /// 1. Initializes the Kira audio manager with default settings
    /// 2. Creates an audio listener at the origin
    /// 3. Loads all required audio files from the assets directory
    /// 4. Starts the background music
    ///
    /// # Returns
    ///
    /// Returns `Ok(GameAudioManager)` on success, or a boxed error if:
    /// - Audio manager initialization fails
    /// - Any audio files cannot be loaded
    /// - Background music fails to start
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The audio system cannot be initialized (no audio device, driver issues)
    /// - Required audio files are missing from the assets directory
    /// - Audio files are in an unsupported format or corrupted
    ///
    /// # Example
    ///
    /// ```rust
    /// let audio_manager = GameAudioManager::new()?;
    /// ```
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut audio_manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

        // Create listener at origin with no rotation
        let listener = audio_manager.add_listener([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0])?;

        // Load all required audio files from embedded assets
        let footstep_data =
            StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_SINGLE_STEP))?;
        let enemy_data =
            StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_SLIME_TRACK))?;
        let complete_data =
            StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_COMPLETE))?;
        let wall_hit_data = StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_WALL))?;
        let select_data = StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_SELECT))?;
        let upgrade_data =
            StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_UPGRADE))?;
        let background_music_data =
            StaticSoundData::from_cursor(std::io::Cursor::new(assets::MUSIC_MAIN_TRACK))?;
        let beeper_rise_data =
            StaticSoundData::from_cursor(std::io::Cursor::new(assets::AUDIO_BEEPER_RISE))?;

        let mut audio_manager_instance = GameAudioManager {
            audio_manager,
            listener,
            footstep_sound: None,
            enemy_sounds: HashMap::new(),
            footstep_data,
            enemy_data,
            complete_data,
            wall_hit_data,
            select_data,
            upgrade_data,
            background_music_data,
            beeper_rise_data,
            background_music_handle: None,
            spatial_tracks: HashMap::new(),
            movement_state: MovementState::Idle,
            wall_hit_cooldown: Duration::from_millis(330),
            last_wall_hit: None,
        };

        // Start background music immediately
        audio_manager_instance.start_background_music()?;

        Ok(audio_manager_instance)
    }

    /// Starts or restarts the background music track.
    ///
    /// This method:
    /// 1. Stops any currently playing background music
    /// 2. Configures the music with appropriate volume (-20dB) and looping
    /// 3. Starts playback of the background music
    ///
    /// The background music is set to loop continuously and plays at a lower
    /// volume to avoid interfering with gameplay audio.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the audio cannot be played.
    ///
    /// # Example
    ///
    /// ```rust
    /// audio_manager.start_background_music()?;
    /// ```
    pub fn start_background_music(&mut self) -> Result<(), Box<dyn Error>> {
        // Stop existing background music if playing
        if let Some(mut handle) = self.background_music_handle.take() {
            handle.stop(Tween::default());
        }

        // Create settings for background music with low volume and looping
        let settings = StaticSoundSettings::new()
            .volume(Decibels::from(-20.0)) // Low volume (-20dB) to not overpower other sounds
            .loop_region(..); // Loop the entire track indefinitely

        // Play the background music with configured settings
        let handle = self
            .audio_manager
            .play(self.background_music_data.clone().with_settings(settings))?;
        self.background_music_handle = Some(handle);

        Ok(())
    }

    /// Restarts the background music from the beginning.
    ///
    /// This is a convenience method that calls `start_background_music()`.
    /// Useful when starting a new game or returning to the main menu.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the music cannot be restarted.
    pub fn restart_background_music(&mut self) -> Result<(), Box<dyn Error>> {
        self.start_background_music()
    }

    /// Adjusts audio volumes for the title screen presentation.
    ///
    /// On the title screen:
    /// - Background music is made louder (-5dB) to be more prominent
    /// - Enemy sounds are made quieter (-10dB) to be less intrusive
    ///
    /// Volume changes are applied with smooth 500ms transitions to avoid
    /// jarring audio changes.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if volume adjustments fail.
    pub fn set_title_screen_volumes(&mut self) -> Result<(), Box<dyn Error>> {
        // Make background music louder and more prominent on title screen
        if let Some(handle) = self.background_music_handle.as_mut() {
            let tween = Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(500), // Smooth transition
                easing: Easing::Linear,
            };
            handle.set_volume(Decibels::from(-5.0), tween);
        }

        // Reduce enemy sound volume on title screen for better focus
        let enemy_ids: Vec<String> = self.enemy_sounds.keys().cloned().collect();
        for enemy_id in enemy_ids {
            if let Some(track) = self.spatial_tracks.get_mut(&enemy_id) {
                let tween = Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_millis(500),
                    easing: Easing::Linear,
                };
                track.set_volume(Decibels::from(-10.0), tween);
            }
        }

        Ok(())
    }

    /// Adjusts audio volumes for the pause menu.
    ///
    /// When the pause menu is open:
    /// - Background music is made much softer (-15dB) to indicate paused state
    /// - Quick 100ms transition provides immediate audio feedback
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if volume adjustment fails.
    pub fn set_pause_menu_volumes(&mut self) -> Result<(), Box<dyn Error>> {
        // Make background music much softer when pause menu is open
        if let Some(handle) = self.background_music_handle.as_mut() {
            let tween = Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(100), // Quick transition for immediate feedback
                easing: Easing::Linear,
            };
            handle.set_volume(Decibels::from(-15.0), tween);
        }

        Ok(())
    }

    /// Resets all audio volumes to normal gameplay levels.
    ///
    /// This method restores:
    /// - Background music to normal volume (-10dB)
    /// - Enemy sounds to full volume (0dB)
    ///
    /// Used when transitioning from title screen or pause menu back to gameplay.
    /// Volume changes are applied with smooth 500ms transitions.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if volume adjustments fail.
    pub fn set_game_volumes(&mut self) -> Result<(), Box<dyn Error>> {
        // Reset background music to normal gameplay volume
        if let Some(handle) = self.background_music_handle.as_mut() {
            let tween = Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(500),
                easing: Easing::Linear,
            };
            handle.set_volume(Decibels::from(-10.0), tween);
        }

        // Reset enemy sounds to full volume for gameplay
        let enemy_ids: Vec<String> = self.enemy_sounds.keys().cloned().collect();
        for enemy_id in enemy_ids {
            if let Some(track) = self.spatial_tracks.get_mut(&enemy_id) {
                let tween = Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_millis(500),
                    easing: Easing::Linear,
                };
                track.set_volume(Decibels::from(0.0), tween); // Full volume
            }
        }

        Ok(())
    }

    /// Starts walking footstep audio.
    ///
    /// Transitions the movement state to `Walking` and begins playing
    /// footstep sounds at normal walking pace (0.5 second loop).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio state change fails.
    pub fn start_walking(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_movement_state(MovementState::Walking)
    }

    /// Starts sprinting footstep audio.
    ///
    /// Transitions the movement state to `Sprinting` and begins playing
    /// footstep sounds at sprint pace (0.25 second loop).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio state change fails.
    pub fn start_sprinting(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_movement_state(MovementState::Sprinting)
    }

    /// Stops all movement-related audio.
    ///
    /// Transitions the movement state to `Idle` and stops any currently
    /// playing footstep sounds.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio state change fails.
    pub fn stop_movement(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_movement_state(MovementState::Idle)
    }

    /// Internal method to change movement state and manage footstep audio.
    ///
    /// This method handles the transition between different movement states:
    /// 1. Stops any currently playing footstep sound
    /// 2. Updates the internal movement state
    /// 3. Starts appropriate footstep audio for the new state
    ///
    /// Different states have different loop timings:
    /// - `Idle`: No footstep audio
    /// - `Walking`: 0.5 second loop for natural walking pace
    /// - `Sprinting`: 0.25 second loop for urgent movement
    ///
    /// # Arguments
    ///
    /// * `new_state` - The movement state to transition to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio operations fail.
    fn set_movement_state(&mut self, new_state: MovementState) -> Result<(), Box<dyn Error>> {
        // Only change state if it's actually different
        if self.movement_state != new_state {
            // Stop current footstep sound if any is playing
            if let Some(mut handle) = self.footstep_sound.take() {
                handle.stop(Tween::default());
            }

            self.movement_state = new_state;

            // Start new footstep sound based on the new state
            match new_state {
                MovementState::Idle => {
                    // No footstep sound for idle state
                }
                MovementState::Walking => {
                    let mut sound_handle = self.audio_manager.play(self.footstep_data.clone())?;
                    sound_handle.set_loop_region(0.0..0.5); // Normal walking speed
                    self.footstep_sound = Some(sound_handle);
                }
                MovementState::Sprinting => {
                    let mut sound_handle = self.audio_manager.play(self.footstep_data.clone())?;
                    sound_handle.set_loop_region(0.0..0.25); // Faster loop for sprinting urgency
                    self.footstep_sound = Some(sound_handle);
                }
            }
        }
        Ok(())
    }

    /// Legacy method for backward compatibility.
    ///
    /// This method is deprecated in favor of `stop_movement()` but is maintained
    /// for existing code compatibility.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if movement cannot be stopped.
    pub fn stop_walking(&mut self) -> Result<(), Box<dyn Error>> {
        self.stop_movement()
    }

    /// Updates the 3D position of the audio listener.
    ///
    /// The listener typically represents the player's position in 3D space.
    /// All spatial audio calculations (distance, direction, attenuation) are
    /// performed relative to this listener position.
    ///
    /// Position changes are smoothly interpolated over 100ms to avoid
    /// jarring audio transitions during rapid movement.
    ///
    /// # Arguments
    ///
    /// * `position` - The new 3D position as [x, y, z] coordinates
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if position update fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Update listener to player position
    /// audio_manager.set_listener_position([player.x, player.y, player.z])?;
    /// ```
    pub fn set_listener_position(&mut self, position: [f32; 3]) -> Result<(), Box<dyn Error>> {
        let tween = Tween {
            start_time: StartTime::Immediate,
            duration: Duration::from_millis(100), // Smooth position interpolation
            easing: Easing::Linear,
        };

        // Update listener position - all spatial tracks automatically update
        // their distance-based effects since they reference this listener
        self.listener.set_position(position, tween);

        Ok(())
    }

    /// Spawns a new enemy with spatial audio at the specified position.
    ///
    /// This method creates a complete spatial audio setup for an enemy:
    /// 1. Creates a spatial audio track with 3D positioning
    /// 2. Configures distance-based volume attenuation (5-3200 units)
    /// 3. Adds reverb effect for spatial realism
    /// 4. Starts looping the enemy audio
    /// 5. Registers the enemy for position updates and removal
    ///
    /// The spatial audio includes:
    /// - **Distance Attenuation**: Volume decreases from +20dB to -50dB over distance
    /// - **3D Positioning**: Audio pans based on relative position to listener
    /// - **Reverb**: Adds environmental audio depth
    /// - **Looping**: Enemy audio loops continuously until removed
    ///
    /// # Arguments
    ///
    /// * `enemy_id` - Unique identifier for this enemy (used for updates/removal)
    /// * `position` - Initial 3D position as [x, y, z] coordinates
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if enemy audio setup fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Spawn enemy at position (10, 0, 5)
    /// audio_manager.spawn_enemy("goblin_1".to_string(), [10.0, 0.0, 5.0])?;
    /// ```
    pub fn spawn_enemy(
        &mut self,
        enemy_id: String,
        position: [f32; 3],
    ) -> Result<(), Box<dyn Error>> {
        // Create spatial track with comprehensive 3D audio setup
        let mut spatial_track = self.audio_manager.add_spatial_sub_track(
            &self.listener,
            position,
            SpatialTrackBuilder::new()
                .spatialization_strength(1.0) // Full 3D effect strength
                .distances(SpatialTrackDistances {
                    min_distance: 1.0,    // Minimum distance for audio calculations
                    max_distance: 3200.0, // Maximum audible distance
                })
                // Add reverb for environmental realism
                .with_effect(ReverbBuilder::new().mix(Value::Fixed(0.3.into())))
                // Volume control based on distance from listener
                .with_effect(VolumeControlBuilder::new(Value::FromListenerDistance(
                    Mapping {
                        input_range: (5.0, 3200.0), // Distance range in world units
                        output_range: ((20.0).into(), (-50.0).into()), // Volume range in dB
                        easing: Easing::OutPowi(3), // Non-linear falloff for realism
                    },
                ))),
        )?;

        // Start playing the looping enemy audio on the spatial track
        let sound_handle = spatial_track.play(self.enemy_data.clone().loop_region(..))?;

        // Register the enemy for future updates and management
        self.spatial_tracks.insert(enemy_id.clone(), spatial_track);
        self.enemy_sounds.insert(enemy_id, sound_handle);
        Ok(())
    }

    /// Updates the 3D position of an existing enemy's audio.
    ///
    /// This method smoothly moves an enemy's spatial audio to a new position
    /// over 100ms. The spatial audio system automatically recalculates:
    /// - Distance-based volume attenuation
    /// - 3D panning and positioning
    /// - Reverb characteristics
    ///
    /// # Arguments
    ///
    /// * `enemy_id` - The unique identifier of the enemy to update
    /// * `position` - New 3D position as [x, y, z] coordinates
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. If the enemy_id doesn't exist, the method
    /// succeeds but performs no action.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Update enemy position as it moves
    /// audio_manager.update_enemy_position("goblin_1", [12.0, 0.0, 3.0])?;
    /// ```
    pub fn update_enemy_position(
        &mut self,
        enemy_id: &str,
        position: [f32; 3],
    ) -> Result<(), Box<dyn Error>> {
        if let Some(track) = self.spatial_tracks.get_mut(enemy_id) {
            let tween = Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(100), // Smooth position transition
                easing: Easing::Linear,
            };
            track.set_position(position, tween);
        }
        Ok(())
    }

    /// Removes an enemy and stops all associated audio.
    ///
    /// This method completely cleans up an enemy's audio:
    /// 1. Stops the enemy's looping audio with a smooth fadeout
    /// 2. Pauses the spatial track
    /// 3. Removes the enemy from internal tracking
    ///
    /// The audio stops with a 100ms fade to avoid abrupt cutoffs.
    ///
    /// # Arguments
    ///
    /// * `enemy_id` - The unique identifier of the enemy to remove
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. If the enemy_id doesn't exist, the method
    /// succeeds but performs no action.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Remove enemy when it dies or despawns
    /// audio_manager.remove_enemy("goblin_1")?;
    /// ```
    pub fn remove_enemy(&mut self, enemy_id: &str) -> Result<(), Box<dyn Error>> {
        let tween = Tween {
            start_time: StartTime::Immediate,
            duration: Duration::from_millis(100), // Smooth fadeout
            easing: Easing::Linear,
        };

        // Stop the enemy's audio handle with fadeout
        if let Some(mut handle) = self.enemy_sounds.remove(enemy_id) {
            handle.stop(tween);
        }

        // Pause and remove the spatial track
        if let Some(mut track) = self.spatial_tracks.remove(enemy_id) {
            track.pause(tween);
        }
        Ok(())
    }

    /// Plays audio data with a specified volume level.
    ///
    /// This is a utility method for playing one-shot sounds (like sound effects)
    /// with precise volume control. The volume is converted from linear scale
    /// (0.0-1.0) to decibels for accurate audio processing.
    ///
    /// # Arguments
    ///
    /// * `audio_data` - The audio data to play
    /// * `volume` - Linear volume level (0.0 = silence, 1.0 = full volume)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio playback fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Play sound effect at half volume
    /// audio_manager.play_with_volume(sound_data, 0.5)?;
    /// ```
    pub fn play_with_volume(
        &mut self,
        audio_data: StaticSoundData,
        volume: f32,
    ) -> Result<(), Box<dyn Error>> {
        // Convert linear volume (0.0-1.0) to decibels for audio processing
        let volume_db = if volume <= 0.0 {
            Decibels::SILENCE // Handle zero/negative volume as silence
        } else {
            volume.into() // Convert to decibels
        };

        // Create sound settings with the specified volume
        let settings = StaticSoundSettings::new().volume(volume_db);

        // Play the sound with volume setting (one-shot, no looping)
        self.audio_manager
            .play(audio_data.with_settings(settings))?;

        Ok(())
    }

    /// Updates the audio manager state.
    ///
    /// This method is called each frame to perform any necessary audio processing.
    /// Currently, most audio calculations (spatial positioning, distance attenuation)
    /// are handled automatically by the Kira library, so this method is mostly
    /// for future extensibility.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio processing fails.
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        // Update method for any necessary audio processing
        // Spatial calculations are handled automatically by kira
        Ok(())
    }

    /// Checks if the player is currently in walking state.
    ///
    /// # Returns
    ///
    /// Returns `true` if movement state is `Walking`, `false` otherwise.
    pub fn is_walking(&self) -> bool {
        self.movement_state == MovementState::Walking
    }

    /// Checks if the player is currently in sprinting state.
    ///
    /// # Returns
    ///
    /// Returns `true` if movement state is `Sprinting`, `false` otherwise.
    pub fn is_sprinting(&self) -> bool {
        self.movement_state == MovementState::Sprinting
    }

    /// Checks if the player is currently moving (walking or sprinting).
    ///
    /// # Returns
    ///
    /// Returns `true` if movement state is not `Idle`, `false` otherwise.
    pub fn is_moving(&self) -> bool {
        self.movement_state != MovementState::Idle
    }

    /// Gets the current movement state.
    ///
    /// # Returns
    ///
    /// Returns the current `MovementState` enum value.
    pub fn get_movement_state(&self) -> MovementState {
        self.movement_state
    }

    /// Gets the number of currently active enemies with spatial audio.
    ///
    /// This count represents enemies that have been spawned but not yet removed.
    /// Useful for debugging and performance monitoring.
    ///
    /// # Returns
    ///
    /// Returns the count of active enemy audio tracks.
    pub fn get_enemy_count(&self) -> usize {
        self.spatial_tracks.len()
    }

    /// Temporarily pauses an enemy's audio.
    ///
    /// The audio can be resumed later with `resume_enemy_audio()`. This is
    /// useful for temporarily disabling enemy audio without fully removing
    /// the enemy (e.g., during cutscenes or special game states).
    ///
    /// # Arguments
    ///
    /// * `enemy_id` - The unique identifier of the enemy to pause
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. If the enemy_id doesn't exist, the method
    /// succeeds but performs no action.
    pub fn pause_enemy_audio(&mut self, enemy_id: &str) -> Result<(), Box<dyn Error>> {
        let tween = Tween {
            start_time: StartTime::Immediate, // or StartTime::Absolute(some_time)
            duration: Duration::from_millis(100), // 100ms transition
            easing: Easing::Linear,           // or other easing functions like Easing::EaseInOut
        };
        if let Some(sound_handle) = self.enemy_sounds.get_mut(enemy_id) {
            sound_handle.pause(tween);
        }
        Ok(())
    }

    /// Resumes a previously paused enemy's audio.
    ///
    /// This method restarts audio playback for an enemy that was paused with
    /// `pause_enemy_audio()`. The audio resumes with a smooth 100ms fade-in
    /// to avoid jarring audio transitions.
    ///
    /// # Arguments
    ///
    /// * `enemy_id` - The unique identifier of the enemy to resume
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. If the enemy_id doesn't exist, the method
    /// succeeds but performs no action.
    pub fn resume_enemy_audio(&mut self, enemy_id: &str) -> Result<(), Box<dyn Error>> {
        let tween = Tween {
            start_time: StartTime::Immediate, // or StartTime::Absolute(some_time)
            duration: Duration::from_millis(100), // 100ms transition
            easing: Easing::Linear,           // or other easing functions like Easing::EaseInOut
        };
        if let Some(sound_handle) = self.enemy_sounds.get_mut(enemy_id) {
            sound_handle.resume(tween);
        }
        Ok(())
    }

    /// Plays the level completion sound effect.
    ///
    /// This method plays a one-shot completion sound at full volume.
    /// Typically used when the player reaches the exit or completes an objective.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio playback fails.
    pub fn complete(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_manager.play(self.complete_data.clone())?;
        Ok(())
    }

    /// Plays the wall collision sound effect with cooldown protection.
    ///
    /// This method plays a wall hit sound when the player collides with walls.
    /// The sound is played at a very low volume (0.0001) and includes a 330ms
    /// cooldown to prevent audio spam when repeatedly hitting walls.
    ///
    /// The cooldown ensures that rapid wall collisions don't create overwhelming
    /// audio feedback while still providing tactile audio response.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio playback fails.
    /// If the cooldown is active, the method returns `Ok(())` without playing sound.
    pub fn wall_hit(&mut self) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();

        // Check if enough time has passed since last hit
        if let Some(last_hit) = self.last_wall_hit {
            if now.duration_since(last_hit) < self.wall_hit_cooldown {
                return Ok(()); // Skip playing sound
            }
        }

        // Play sound at 1/3 volume (0.33)
        self.play_with_volume(self.wall_hit_data.clone(), 0.0001)?;
        self.last_wall_hit = Some(now);
        Ok(())
    }

    /// Plays the UI selection sound effect.
    ///
    /// This method plays a one-shot selection sound at full volume.
    /// Typically used for menu navigation, button clicks, or UI interactions.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio playback fails.
    pub fn play_select(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_manager.play(self.select_data.clone())?;
        Ok(())
    }

    /// Plays the upgrade/power-up sound effect.
    ///
    /// This method plays a one-shot upgrade sound at full volume.
    /// Typically used when the player collects power-ups, upgrades, or
    /// gains new abilities.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio playback fails.
    pub fn play_upgrade(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_manager.play(self.upgrade_data.clone())?;
        Ok(())
    }

    /// Plays the beeper-rise sound effect.
    ///
    /// This method plays a one-shot beeper-rise sound at full volume.
    /// Typically used for rising tones, alerts, or ascending audio cues.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if audio playback fails.
    pub fn play_beeper_rise(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_manager.play(self.beeper_rise_data.clone())?;
        Ok(())
    }
}
