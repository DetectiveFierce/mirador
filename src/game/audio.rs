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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementState {
    Idle,
    Walking,
    Sprinting,
}

pub struct GameAudioManager {
    audio_manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
    footstep_sound: Option<StaticSoundHandle>,
    enemy_sounds: HashMap<String, StaticSoundHandle>,
    footstep_data: StaticSoundData,
    enemy_data: StaticSoundData,
    complete_data: StaticSoundData,
    wall_hit_data: StaticSoundData,
    spatial_tracks: HashMap<String, SpatialTrackHandle>,
    movement_state: MovementState,
    wall_hit_cooldown: Duration,
    last_wall_hit: Option<Instant>,
}

impl GameAudioManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut audio_manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

        // Create listener at origin
        let listener = audio_manager.add_listener([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0])?;

        // Load audio files
        let footstep_data = StaticSoundData::from_file("assets/audio/single_step.wav")?;
        let enemy_data = StaticSoundData::from_file("assets/audio/jeffree-star-asmr.ogg")?;
        let complete_data = StaticSoundData::from_file("assets/audio/complete.wav")?;
        let wall_hit_data = StaticSoundData::from_file("assets/audio/wall.wav")?;

        Ok(GameAudioManager {
            audio_manager,
            listener,
            footstep_sound: None,
            enemy_sounds: HashMap::new(),
            footstep_data,
            enemy_data,
            complete_data,
            wall_hit_data,
            spatial_tracks: HashMap::new(),
            movement_state: MovementState::Idle,
            wall_hit_cooldown: Duration::from_millis(330),
            last_wall_hit: None,
        })
    }

    pub fn start_walking(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_movement_state(MovementState::Walking)
    }

    pub fn start_sprinting(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_movement_state(MovementState::Sprinting)
    }

    pub fn stop_movement(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_movement_state(MovementState::Idle)
    }

    fn set_movement_state(&mut self, new_state: MovementState) -> Result<(), Box<dyn Error>> {
        if self.movement_state != new_state {
            // Stop current footstep sound if any
            if let Some(mut handle) = self.footstep_sound.take() {
                handle.stop(Tween::default());
            }

            self.movement_state = new_state;

            // Start new footstep sound based on state
            match new_state {
                MovementState::Idle => {
                    // No footstep sound for idle
                }
                MovementState::Walking => {
                    let mut sound_handle = self.audio_manager.play(self.footstep_data.clone())?;
                    sound_handle.set_loop_region(0.0..0.5); // Normal walking speed
                    self.footstep_sound = Some(sound_handle);
                }
                MovementState::Sprinting => {
                    let mut sound_handle = self.audio_manager.play(self.footstep_data.clone())?;
                    sound_handle.set_loop_region(0.0..0.25); // Faster loop for sprinting
                    self.footstep_sound = Some(sound_handle);
                }
            }
        }
        Ok(())
    }

    // Legacy method for backward compatibility
    pub fn stop_walking(&mut self) -> Result<(), Box<dyn Error>> {
        self.stop_movement()
    }

    pub fn set_listener_position(&mut self, position: [f32; 3]) -> Result<(), Box<dyn Error>> {
        let tween = Tween {
            start_time: StartTime::Immediate,
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
        };

        // Update listener position - spatial tracks will automatically update
        // their distance-based effects since they reference the listener
        self.listener.set_position(position, tween);

        Ok(())
    }

    pub fn spawn_enemy(
        &mut self,
        enemy_id: String,
        position: [f32; 3],
    ) -> Result<(), Box<dyn Error>> {
        let mut spatial_track = self.audio_manager.add_spatial_sub_track(
            &self.listener,
            position,
            SpatialTrackBuilder::new()
                .spatialization_strength(1.0)
                .distances(SpatialTrackDistances {
                    min_distance: 1.0,
                    max_distance: 3200.0,
                })
                .with_effect(ReverbBuilder::new().mix(Value::Fixed(0.3.into())))
                .with_effect(VolumeControlBuilder::new(Value::FromListenerDistance(
                    Mapping {
                        input_range: (5.0, 3200.0),
                        output_range: ((20.0).into(), (-50.0).into()),
                        easing: Easing::OutPowi(3),
                    },
                ))),
        )?;

        let sound_handle = spatial_track.play(self.enemy_data.clone().loop_region(0.0..1089.0))?;

        self.spatial_tracks.insert(enemy_id.clone(), spatial_track);
        self.enemy_sounds.insert(enemy_id, sound_handle);
        Ok(())
    }

    pub fn update_enemy_position(
        &mut self,
        enemy_id: &str,
        position: [f32; 3],
    ) -> Result<(), Box<dyn Error>> {
        if let Some(track) = self.spatial_tracks.get_mut(enemy_id) {
            let tween = Tween {
                start_time: StartTime::Immediate, // or StartTime::Absolute(some_time)
                duration: Duration::from_millis(100), // 100ms transition
                easing: Easing::Linear, // or other easing functions like Easing::EaseInOut
            };
            track.set_position(position, tween);
        }
        Ok(())
    }

    pub fn remove_enemy(&mut self, enemy_id: &str) -> Result<(), Box<dyn Error>> {
        let tween = Tween {
            start_time: StartTime::Immediate, // or StartTime::Absolute(some_time)
            duration: Duration::from_millis(100), // 100ms transition
            easing: Easing::Linear,           // or other easing functions like Easing::EaseInOut
        };
        if let Some(mut handle) = self.enemy_sounds.remove(enemy_id) {
            handle.stop(tween);
        }
        if let Some(mut track) = self.spatial_tracks.remove(enemy_id) {
            track.pause(tween);
        }
        Ok(())
    }

    pub fn play_with_volume(
        &mut self,
        audio_data: StaticSoundData,
        volume: f32,
    ) -> Result<(), Box<dyn Error>> {
        // Convert linear volume (0.0-1.0) to decibels
        let volume_db = if volume <= 0.0 {
            Decibels::SILENCE
        } else {
            volume.into()
        };

        // Create sound settings with the specified volume
        let settings = StaticSoundSettings::new().volume(volume_db);

        // Play the sound with the volume setting
        self.audio_manager
            .play(audio_data.with_settings(settings))?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        // Update method for any necessary audio processing
        // Spatial calculations are handled automatically by kira
        Ok(())
    }

    pub fn is_walking(&self) -> bool {
        self.movement_state == MovementState::Walking
    }

    pub fn is_sprinting(&self) -> bool {
        self.movement_state == MovementState::Sprinting
    }

    pub fn is_moving(&self) -> bool {
        self.movement_state != MovementState::Idle
    }

    pub fn get_movement_state(&self) -> MovementState {
        self.movement_state
    }

    pub fn get_enemy_count(&self) -> usize {
        self.spatial_tracks.len()
    }

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

    /// Play the completion sound effect
    pub fn complete(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_manager.play(self.complete_data.clone())?;
        Ok(())
    }

    /// Play the wall hit sound effect
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
}
