use kira::Easing;
use kira::Mapping;
use kira::StartTime;
use kira::Tween;
use kira::Value;
use kira::effect::reverb::ReverbBuilder;
use kira::effect::volume_control::VolumeControlBuilder;
use kira::listener::ListenerHandle;
use kira::sound::static_sound::StaticSoundData;
use kira::sound::static_sound::StaticSoundHandle;
use kira::track::SpatialTrackBuilder;
use kira::track::SpatialTrackDistances;
use kira::track::SpatialTrackHandle;
use kira::{AudioManager, AudioManagerSettings, DefaultBackend};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
pub struct GameAudioManager {
    audio_manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
    footstep_sound: Option<StaticSoundHandle>,
    enemy_sounds: HashMap<String, StaticSoundHandle>,
    footstep_data: StaticSoundData,
    enemy_data: StaticSoundData,
    spatial_tracks: HashMap<String, SpatialTrackHandle>,
    is_walking: bool,
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

        Ok(GameAudioManager {
            audio_manager,
            listener,
            footstep_sound: None,
            enemy_sounds: HashMap::new(),
            footstep_data,
            enemy_data,
            spatial_tracks: HashMap::new(),
            is_walking: false,
        })
    }

    pub fn start_walking(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.is_walking {
            self.is_walking = true;
            let mut sound_handle = self.audio_manager.play(self.footstep_data.clone())?;
            sound_handle.set_loop_region(0.0..0.5);
            self.footstep_sound = Some(sound_handle);
        }
        Ok(())
    }

    pub fn stop_walking(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_walking {
            self.is_walking = false;
            if let Some(mut handle) = self.footstep_sound.take() {
                handle.stop(kira::Tween::default());
            }
        }
        Ok(())
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

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        // Update method for any necessary audio processing
        // Spatial calculations are handled automatically by kira
        Ok(())
    }

    pub fn is_walking(&self) -> bool {
        self.is_walking
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
}
