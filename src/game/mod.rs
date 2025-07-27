//! Game state management module.
//!
//! # Overview
//!
//! This module provides the core game state management functionality for a maze-based game.
//! It defines the main [`GameState`] struct that tracks all mutable state during the game loop,
//! including player state, timing information, UI elements, audio management, and game progression.
//!
//! # Key Components
//!
//! - **Player Management**: Tracks player position, orientation, and state
//! - **Timing System**: Handles frame timing, FPS calculation, and game timers
//! - **UI Management**: Manages game interface elements like timers, scores, and level displays
//! - **Audio Integration**: Coordinates game audio through the audio manager
//! - **Screen Management**: Handles different game screens (title, game, pause, etc.)
//! - **Collision System**: Manages entity collision detection and response
//!
//! # Timer Decimal Alignment
//!
//! A key feature of this module is the precise decimal alignment of the game timer display.
//! The timer's decimal point is always positioned at the exact horizontal center of the screen
//! by measuring the width of the timer string up to and including the decimal point, then
//! offsetting the x position accordingly. This creates a visually stable timer display
//! where the decimal point never moves as the numbers change.
//!
//! # Usage Example
//!
//! ```rust
//! use game_state::GameState;
//!
//! // Create a new game state with default values
//! let mut game_state = GameState::new();
//!
//! // Start a game timer with custom configuration
//! game_state.start_game_timer(Some(TimerConfig {
//!     duration: Duration::from_secs(60),
//!     warning_threshold: Duration::from_secs(20),
//!     critical_threshold: Duration::from_secs(10),
//!     ..Default::default()
//! }));
//!
//! // Update game state each frame
//! game_state.set_level(2);
//! game_state.set_score(1500);
//! ```

// Timer decimal alignment: The timer's decimal point is always aligned with the vertical center of the screen by measuring the width of the timer string up to and including the decimal and offsetting the x position accordingly. See initialize_game_ui and update_game_ui for details.
pub mod audio;
pub mod collision;
pub mod enemy;
pub mod keys;
pub mod maze;
pub mod player;
pub mod upgrades;

use self::audio::GameAudioManager;
use self::collision::CollisionSystem;
use self::player::Player;
use crate::game::enemy::Enemy;
use crate::game::maze::generator::Cell;
use crate::renderer::text::TextPosition;
use crate::renderer::text::TextRenderer;
use crate::renderer::text::TextStyle;
use glyphon::Color;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::window::Window;

/// Represents the entire mutable state of the game.
///
/// This struct serves as the central hub for all game state information and is updated
/// every frame during the game loop. It encapsulates all the necessary components
/// for running a complete game session.
///
/// # Fields Overview
///
/// - **Player State**: Current player position, orientation, and movement
/// - **Timing**: Frame timing, delta time calculation, and FPS tracking
/// - **UI State**: Current screen, mouse capture status, and UI elements
/// - **Game Logic**: Maze path, collision detection, exit conditions
/// - **Audio**: Background music, sound effects, and spatial audio
/// - **Game Progression**: Level, score, and timer management
///
/// # Lifecycle
///
/// The GameState is typically created once at game startup using [`GameState::new()`],
/// then continuously updated throughout the game loop. Different screens may modify
/// different subsets of the state.
pub struct GameState {
    /// The player character containing position, orientation, movement state, and camera data.
    ///
    /// This includes the player's 3D position in the maze, viewing direction, movement velocity,
    /// and any player-specific flags or states.
    pub player: Player,

    /// Timestamp of the previous frame, used for delta time calculation.
    ///
    /// This is essential for frame-rate independent movement and animations.
    /// Updated every frame in the main game loop.
    pub last_frame_time: Instant,

    /// Time elapsed since the last frame in seconds.
    ///
    /// Calculated as the difference between current time and `last_frame_time`.
    /// Used for smooth, frame-rate independent game logic updates.
    pub delta_time: f32,

    /// Total number of frames rendered since the game started.
    ///
    /// Used for debugging, profiling, and any frame-based logic that needs
    /// to track total elapsed frames rather than time.
    pub frame_count: u32,

    /// Current frames per second, updated periodically.
    ///
    /// Calculated by counting frames over a time window and updated
    /// at regular intervals defined by `last_fps_time`.
    pub current_fps: u32,

    /// Timestamp of the last FPS calculation update.
    ///
    /// FPS is not calculated every frame for performance reasons, but rather
    /// at regular intervals (typically every second).
    pub last_fps_time: Instant,

    /// File system path to the currently loaded maze definition.
    ///
    /// `None` if no maze is currently loaded. When `Some`, contains the path
    /// to the maze file that was used to generate the current game level.
    pub maze_path: Option<PathBuf>,

    /// Whether mouse input is captured for camera movement.
    ///
    /// When `true`, mouse movement controls the camera/player view direction.
    /// When `false`, the mouse cursor is free to interact with UI elements.
    /// Typically disabled on menus and enabled during gameplay.
    pub capture_mouse: bool,

    /// System responsible for detecting and resolving collisions between game entities.
    ///
    /// Handles collision detection between the player and maze walls, enemies,
    /// pickups, and other interactive elements in the game world.
    pub collision_system: CollisionSystem,

    /// Whether the player has reached the maze exit.
    ///
    /// Set to `true` when the player successfully navigates to the exit cell.
    /// Triggers end-game sequences and screen transitions.
    pub exit_reached: bool,

    /// The specific maze cell that serves as the exit point.
    ///
    /// `None` if no exit is defined or the maze hasn't been loaded.
    /// Used for collision detection and visual highlighting of the exit.
    pub exit_cell: Option<Cell>,

    /// Manager for all game UI elements including timers, scores, and levels.
    ///
    /// Centralizes UI state management and provides a clean interface
    /// for updating display elements throughout the game.
    pub game_ui: GameUIManager,

    /// The currently active screen or game mode.
    ///
    /// Determines which input handlers are active, which rendering
    /// operations are performed, and which game logic updates are executed.
    pub current_screen: CurrentScreen,

    /// The screen that was active before entering the current screen.
    ///
    /// Primarily used for pause menu functionality to remember which
    /// screen to return to when unpausing. `None` if no previous screen
    /// is recorded or applicable.
    pub previous_screen: Option<CurrentScreen>,

    /// The enemy entity in the game world.
    ///
    /// Contains enemy position, AI state, movement patterns, and any
    /// enemy-specific behavior flags. Currently supports a single enemy
    /// but could be extended to support multiple enemies.
    pub enemy: Enemy,

    /// Centralized audio management system.
    ///
    /// Handles background music, sound effects, spatial audio positioning,
    /// and volume control. Manages audio state transitions between different
    /// game screens and situations.
    pub audio_manager: GameAudioManager,

    /// Flag indicating whether the game is running in test mode.
    ///
    /// Test mode may enable debugging features, disable certain game
    /// mechanics, or provide additional information for development purposes.
    pub is_test_mode: bool,

    /// Timer tracking upward movement animation when exit is reached.
    ///
    /// When the player reaches the exit, this timer counts down from 3 seconds
    /// during which the player character moves upward as part of the exit animation.
    pub exit_reached_timer: f32,

    /// Flag to ensure the beeper rise sound effect is only played once.
    ///
    /// Prevents the audio from being triggered multiple times during
    /// the exit sequence animation.
    pub beeper_rise_played: bool,
}

/// Represents the current state of the pause menu.
///
/// Used to track whether the pause menu is currently displayed to the player
/// or hidden. This affects input handling and rendering behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuState {
    /// Pause menu is currently visible and active
    Open,
    /// Pause menu is hidden, game continues normally
    Closed,
}

/// Represents the different screens or states the game can be in.
///
/// Each screen has its own input handling, rendering logic, and state management.
/// The current screen determines which game systems are active and how user
/// input is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    /// Initial screen shown when the game starts, typically with menu options
    Title,
    /// Temporary screen shown while loading game assets or generating maze
    Loading,
    /// Main gameplay screen where the player navigates the maze
    Game,
    /// Overlay screen shown when the game is paused, with resume/quit options
    Pause,
    /// Screen displayed when the player fails (e.g., time runs out, caught by enemy)
    GameOver,
    /// Screen for starting a new game, possibly with difficulty selection
    NewGame,
    /// Screen for selecting or purchasing player upgrades between levels
    UpgradeMenu,
    /// Victory screen shown when the player successfully reaches the maze exit
    ExitReached,
}

impl Default for GameState {
    /// Returns a new [`GameState`] with default values.
    ///
    /// This is equivalent to calling [`GameState::new()`] and is provided
    /// for convenience when using derive macros or generic code that expects
    /// a Default implementation.
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Creates a new [`GameState`] with initialized default values.
    ///
    /// This constructor performs several important initialization steps:
    ///
    /// 1. **Audio System**: Initializes the audio manager and spawns the enemy audio source
    /// 2. **Player Setup**: Creates a new player at the default starting position
    /// 3. **Timing**: Sets up frame timing and FPS tracking systems
    /// 4. **Collision System**: Configures collision detection with appropriate player dimensions
    /// 5. **UI Management**: Initializes the game UI manager for timers, scores, and levels
    /// 6. **Screen State**: Sets the initial screen to the title screen
    /// 7. **Enemy Setup**: Creates and positions the enemy entity
    /// 8. **Audio Configuration**: Sets appropriate volume levels for the title screen
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The audio manager fails to initialize (missing audio drivers, etc.)
    /// - The enemy audio source cannot be spawned
    /// - Title screen audio volumes cannot be set
    ///
    /// # Examples
    ///
    /// ```rust
    /// let game_state = GameState::new();
    /// assert_eq!(game_state.current_screen, CurrentScreen::Title);
    /// assert!(!game_state.exit_reached);
    /// assert_eq!(game_state.game_ui.get_level(), 1);
    /// ```
    pub fn new() -> Self {
        use crate::benchmarks::{BenchmarkConfig, Profiler};

        // Initialize profiler for GameState initialization benchmarking
        let mut init_profiler = Profiler::new(BenchmarkConfig {
            enabled: true,
            print_results: false, // Respect user's console output preference
            write_to_file: false,
            min_duration_threshold: std::time::Duration::from_micros(1),
            max_samples: 1000,
        });

        // Benchmark audio manager initialization (most taxing part)
        init_profiler.start_section("audio_manager_initialization");
        let mut audio_manager =
            GameAudioManager::new().expect("Failed to initialize audio manager");
        init_profiler.end_section("audio_manager_initialization");

        // Benchmark enemy audio source spawning
        init_profiler.start_section("enemy_audio_source_spawning");
        audio_manager
            .spawn_enemy("enemy".to_string(), [-0.5, 30.0, 0.0])
            .expect("Failed to spawn enemy");
        init_profiler.end_section("enemy_audio_source_spawning");

        // Benchmark player creation
        init_profiler.start_section("player_creation");
        let player = Player::new();
        init_profiler.end_section("player_creation");

        // Benchmark collision system initialization
        init_profiler.start_section("collision_system_init");
        let collision_system = CollisionSystem::new(
            5.0,   // player_radius - horizontal collision boundary
            100.0, // player_height - vertical collision boundary
        );
        init_profiler.end_section("collision_system_init");

        // Benchmark enemy creation
        init_profiler.start_section("enemy_creation");
        let enemy = Enemy::new([-0.5, 30.0, 0.0], 150.0);
        init_profiler.end_section("enemy_creation");

        let mut game_state = Self {
            // Initialize player at default starting position with default orientation
            player,

            // Set up timing system - these will be properly updated on the first frame
            last_frame_time: Instant::now(),
            delta_time: 0.0,
            frame_count: 0,
            current_fps: 0,
            last_fps_time: Instant::now(),

            // No maze loaded initially
            maze_path: None,

            // Start with mouse captured for immediate gameplay readiness
            capture_mouse: true,

            // Initialize collision system with player dimensions
            // These values should match the actual player model dimensions
            collision_system,

            // Game starts with exit not reached
            exit_reached: false,
            exit_cell: None,

            // Initialize UI management system
            game_ui: GameUIManager::new(),

            // Start on the title screen
            current_screen: CurrentScreen::Title,
            previous_screen: None,

            // Create enemy at specified starting position with movement speed
            enemy,

            // Audio manager was initialized above
            audio_manager,

            // Start in normal (non-test) mode
            is_test_mode: false,

            // Exit animation not active initially
            exit_reached_timer: 0.0,
            beeper_rise_played: false,
        };

        // Benchmark title screen audio configuration
        init_profiler.start_section("title_audio_config");
        game_state
            .audio_manager
            .set_title_screen_volumes()
            .expect("Failed to set title screen volumes");
        init_profiler.end_section("title_audio_config");

        game_state
    }

    /// Starts the game timer with optional custom configuration.
    ///
    /// This method initializes and starts a new countdown timer for the current game session.
    /// If no configuration is provided, default timer settings are used.
    ///
    /// # Parameters
    ///
    /// * `config` - Optional timer configuration specifying duration, warning thresholds,
    ///              and color schemes. If `None`, uses [`TimerConfig::default()`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Start timer with default 30-second configuration
    /// game_state.start_game_timer(None);
    ///
    /// // Start timer with custom 60-second configuration
    /// let custom_config = TimerConfig {
    ///     duration: Duration::from_secs(60),
    ///     warning_threshold: Duration::from_secs(20),
    ///     critical_threshold: Duration::from_secs(10),
    ///     ..Default::default()
    /// };
    /// game_state.start_game_timer(Some(custom_config));
    /// ```
    pub fn start_game_timer(&mut self, config: Option<TimerConfig>) {
        self.game_ui.start_timer(config);
    }

    /// Stops the currently running game timer.
    ///
    /// The timer will cease counting down and maintain its current remaining time.
    /// This is different from pausing, as the timer cannot be resumed after stopping.
    /// To restart timing, use [`start_game_timer`] again.
    ///
    /// If no timer is currently running, this method has no effect.
    pub fn stop_game_timer(&mut self) {
        self.game_ui.stop_timer();
    }

    /// Resets the game timer to its initial configured duration and restarts it.
    ///
    /// This method preserves the timer's original configuration (duration, colors, thresholds)
    /// but resets the countdown to the full duration and begins timing again.
    ///
    /// If no timer exists, this method has no effect.
    ///
    /// # Use Cases
    ///
    /// - Starting a new level while keeping the same timer settings
    /// - Restarting after a game over
    /// - Resetting during testing or debugging
    pub fn reset_game_timer(&mut self) {
        self.game_ui.reset_timer();
    }

    /// Checks whether the game timer has expired (reached zero).
    ///
    /// # Returns
    ///
    /// `true` if the timer exists and has reached zero, `false` if the timer
    /// is still running or no timer exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// if game_state.is_game_timer_expired() {
    ///     // Handle timeout condition (e.g., game over, move to next level)
    ///     game_state.current_screen = CurrentScreen::GameOver;
    /// }
    /// ```
    pub fn is_game_timer_expired(&self) -> bool {
        self.game_ui.is_timer_expired()
    }

    /// Updates the current game level.
    ///
    /// This method updates the level counter displayed in the game UI.
    /// The level affects gameplay difficulty, timer duration, enemy behavior,
    /// and other game mechanics.
    ///
    /// # Parameters
    ///
    /// * `level` - The new level number (typically starts at 1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Advance to level 2
    /// game_state.set_level(2);
    ///
    /// // Reset to level 1 for new game
    /// game_state.set_level(1);
    /// ```
    pub fn set_level(&mut self, level: i32) {
        self.game_ui.set_level(level);
    }

    /// Updates the current game score.
    ///
    /// The score is displayed in the game UI and typically increases based on
    /// player performance, time remaining, items collected, or other gameplay factors.
    ///
    /// # Parameters
    ///
    /// * `score` - The new score value
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Set initial score
    /// game_state.set_score(0);
    ///
    /// // Add points for completing a level
    /// let current_score = game_state.game_ui.get_score();
    /// game_state.set_score(current_score + 1000);
    /// ```
    pub fn set_score(&mut self, score: u32) {
        self.game_ui.set_score(score);
    }
}

/// Configuration settings for game timers.
///
/// This struct defines all the parameters needed to create a countdown timer,
/// including the total duration, visual warning thresholds, and color schemes
/// for different timer states.
///
/// # Color Transitions
///
/// The timer changes color based on remaining time:
/// - **Normal**: More time than warning threshold (typically green)
/// - **Warning**: Between warning and critical thresholds (typically yellow)
/// - **Critical**: Less than critical threshold remaining (typically red)
#[derive(Debug, Clone)]
pub struct TimerConfig {
    /// Total duration of the timer countdown.
    pub duration: Duration,

    /// Time remaining when the timer should switch to warning color.
    ///
    /// When the remaining time drops to or below this threshold,
    /// the timer display changes to the warning color to alert the player.
    pub warning_threshold: Duration,

    /// Time remaining when the timer should switch to critical color.
    ///
    /// When the remaining time drops to or below this threshold,
    /// the timer display changes to the critical color to urgently
    /// warn the player that time is almost up.
    pub critical_threshold: Duration,

    /// Color used when plenty of time remains (above warning threshold).
    pub normal_color: Color,

    /// Color used when time is getting low (between warning and critical).
    pub warning_color: Color,

    /// Color used when time is almost expired (below critical threshold).
    pub critical_color: Color,
}

impl Default for TimerConfig {
    /// Creates a default timer configuration with reasonable settings.
    ///
    /// Default settings:
    /// - **Duration**: 30 seconds
    /// - **Warning threshold**: 20 seconds (green to yellow transition)
    /// - **Critical threshold**: 10 seconds (yellow to red transition)
    /// - **Colors**: Green (normal), Yellow (warning), Red (critical)
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(30),
            warning_threshold: Duration::from_secs(20),
            critical_threshold: Duration::from_secs(10),
            normal_color: Color::rgb(100, 255, 100), // Light green
            warning_color: Color::rgb(255, 255, 100), // Light yellow
            critical_color: Color::rgb(255, 100, 100), // Light red
        }
    }
}

/// Internal timer implementation that handles countdown logic and state tracking.
///
/// This struct manages the actual countdown mechanics, pause/resume functionality,
/// color transitions, and time formatting. It's used internally by [`GameUIManager`]
/// and is not typically accessed directly by game code.
///
/// # Timer States
///
/// - **Stopped**: Timer is not running and will not count down
/// - **Running**: Timer is actively counting down
/// - **Paused**: Timer is temporarily stopped but can be resumed
/// - **Expired**: Timer has reached zero and stopped automatically
#[derive(Debug)]
pub struct GameTimer {
    /// The moment when the timer was started (or last restarted).
    pub start_time: Instant,

    /// Configuration defining duration, thresholds, and colors.
    pub config: TimerConfig,

    /// Whether the timer is currently counting down.
    pub is_running: bool,

    /// Whether the timer has reached zero.
    pub is_expired: bool,

    /// If paused, the moment when pause was activated.
    ///
    /// `None` if the timer is not currently paused.
    pub paused_at: Option<Instant>,

    /// Total time that has been spent in paused state.
    ///
    /// This is subtracted from elapsed time calculations to ensure
    /// that paused time doesn't count against the timer duration.
    pub elapsed_paused: Duration,
}

impl GameTimer {
    /// Creates a new timer with the specified configuration.
    ///
    /// The timer is created in a stopped state and must be started
    /// with [`start()`] to begin counting down.
    ///
    /// # Parameters
    ///
    /// * `config` - Timer configuration including duration and color settings
    pub fn new(config: TimerConfig) -> Self {
        Self {
            start_time: Instant::now(),
            config,
            is_running: false,
            is_expired: false,
            paused_at: None,
            elapsed_paused: Duration::ZERO,
        }
    }

    /// Starts or restarts the timer countdown.
    ///
    /// This method resets all timer state and begins counting down from
    /// the configured duration. If the timer was previously paused or expired,
    /// it will be reset to a fresh state.
    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.is_running = true;
        self.is_expired = false;
        self.paused_at = None;
        self.elapsed_paused = Duration::ZERO;
    }

    /// Pauses the timer if it's currently running.
    ///
    /// While paused, the timer will not count down and its display
    /// will remain frozen. The timer can be resumed with [`resume()`].
    ///
    /// If the timer is already paused or not running, this method has no effect.
    pub fn pause(&mut self) {
        if self.is_running && self.paused_at.is_none() {
            self.paused_at = Some(Instant::now());
        }
    }

    /// Resumes the timer from a paused state.
    ///
    /// This method calculates how long the timer was paused and adds that
    /// duration to the total paused time, ensuring that paused time doesn't
    /// count against the timer duration.
    ///
    /// If the timer is not currently paused, this method has no effect.
    pub fn resume(&mut self) {
        if let Some(paused_at) = self.paused_at.take() {
            self.elapsed_paused += paused_at.elapsed();
        }
    }

    /// Stops the timer immediately.
    ///
    /// Unlike pausing, a stopped timer cannot be resumed. To restart timing,
    /// use [`start()`] which will reset the timer to its full duration.
    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Resets the timer to its initial state without starting it.
    ///
    /// This clears the expired flag and resets pause state, but does not
    /// automatically start the timer. Call [`start()`] to begin countdown.
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.is_expired = false;
        self.paused_at = None;
        self.elapsed_paused = Duration::ZERO;
    }

    /// Calculates and returns the time remaining on the timer.
    ///
    /// This method accounts for paused time to ensure accurate remaining time
    /// calculation. If the timer is expired, stopped, or has no time remaining,
    /// returns [`Duration::ZERO`].
    ///
    /// # Returns
    ///
    /// The amount of time left before the timer expires, or zero if the timer
    /// is not running or has already expired.
    pub fn get_remaining_time(&self) -> Duration {
        // If timer is not running or already expired, no time remains
        if !self.is_running || self.is_expired {
            return Duration::ZERO;
        }

        // Calculate elapsed time, accounting for current pause state
        let elapsed = if let Some(paused_at) = self.paused_at {
            // Timer is currently paused, so use pause time as the end point
            paused_at.duration_since(self.start_time) - self.elapsed_paused
        } else {
            // Timer is running, use current time as the end point
            Instant::now().duration_since(self.start_time) - self.elapsed_paused
        };

        // Subtract elapsed time from total duration, ensuring we don't go negative
        self.config
            .duration
            .checked_sub(elapsed)
            .unwrap_or(Duration::ZERO)
    }

    /// Checks if the timer has expired (reached zero).
    ///
    /// # Returns
    ///
    /// `true` if the timer has reached zero or is stopped with no time remaining,
    /// `false` if there is still time left.
    pub fn is_expired(&self) -> bool {
        self.is_expired || (!self.is_running && self.get_remaining_time().is_zero())
    }

    /// Updates the timer state and checks for expiration.
    ///
    /// This method should be called every frame to update the timer's internal
    /// state. It checks if the timer has reached zero and updates the expired flag.
    ///
    /// # Returns
    ///
    /// `true` if the timer just expired this frame (transition from running to expired),
    /// `false` if the timer was already expired or is still running.
    ///
    /// This return value is useful for triggering one-time events when the timer expires.
    pub fn update(&mut self) -> bool {
        // Don't update if not running or currently paused
        if !self.is_running || self.paused_at.is_some() {
            return false;
        }

        let remaining = self.get_remaining_time();
        let was_expired = self.is_expired;
        self.is_expired = remaining.is_zero();

        // Return true only if the timer just expired this frame
        !was_expired && self.is_expired
    }

    /// Determines the appropriate color for the timer display based on remaining time.
    ///
    /// The color changes based on the configured thresholds:
    /// - Normal color when above warning threshold
    /// - Warning color when between warning and critical thresholds  
    /// - Critical color when below critical threshold
    ///
    /// # Returns
    ///
    /// The [`Color`] that should be used for displaying the timer.
    pub fn get_current_color(&self) -> Color {
        let remaining = self.get_remaining_time();
        if remaining <= self.config.critical_threshold {
            self.config.critical_color
        } else if remaining <= self.config.warning_threshold {
            self.config.warning_color
        } else {
            self.config.normal_color
        }
    }

    /// Formats the remaining time as a string for display.
    ///
    /// The time is formatted as "MM.SS" where MM is minutes (or seconds if < 1 minute)
    /// and SS is fractional seconds to two decimal places. This provides a precise,
    /// easily readable timer display.
    ///
    /// # Returns
    ///
    /// A formatted string showing the remaining time, e.g., "23.45", "01.20", "00.00"
    ///
    /// # Examples
    ///
    /// - 23.45 seconds remaining: "23.45"
    /// - 1 minute 20.5 seconds remaining: "80.50"  
    /// - Timer expired: "00.00"
    pub fn format_time(&self) -> String {
        let remaining = self.get_remaining_time();
        let seconds = remaining.as_secs_f64();
        format!("{:05.2}", seconds)
    }
}

/// Manages all game UI elements including timers, scores, and level displays.
///
/// This struct centralizes the management of game interface elements and provides
/// a clean API for updating UI state throughout the game. It handles the lifecycle
/// of timers and maintains display formatting for scores and levels.
///
/// # Responsibilities
///
/// - **Timer Management**: Creating, starting, stopping, and updating game timers
/// - **Score Tracking**: Maintaining and formatting the current game score
/// - **Level Display**: Tracking and formatting the current game level
/// - **UI State**: Coordinating updates to all UI elements
pub struct GameUIManager {
    /// The currently active game timer, if any.
    ///
    /// `None` if no timer is currently configured. When `Some`, contains
    /// a timer that handles countdown logic and display formatting.
    pub timer: Option<GameTimer>,

    /// The current game level (typically starts at 1).
    pub level: i32,

    /// The current game score.
    pub score: u32,
}

impl Default for GameUIManager {
    /// Creates a new GameUIManager with default values.
    fn default() -> Self {
        Self::new()
    }
}

impl GameUIManager {
    /// Creates a new GameUIManager with initial values.
    ///
    /// Initializes with no active timer, level 1, and score 0.
    pub fn new() -> Self {
        Self {
            timer: None,
            level: 1,
            score: 0,
        }
    }

    /// Creates and starts a new game timer with the specified configuration.
    ///
    /// If a timer already exists, it will be replaced by the new timer.
    /// The new timer starts immediately upon creation.
    ///
    /// # Parameters
    ///
    /// * `config` - Optional timer configuration. If `None`, uses default settings.
    pub fn start_timer(&mut self, config: Option<TimerConfig>) {
        let config = config.unwrap_or_default();
        let mut timer = GameTimer::new(config);
        timer.start();
        self.timer = Some(timer);
    }

    /// Stops the currently running timer.
    ///
    /// The timer will cease counting down but will remain available for
    /// future resumption.
    pub fn stop_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.stop();
        }
    }

    /// Resets the game timer to its initial configured duration and restarts it.
    ///
    /// If a timer exists, this method resets its countdown and starts it again.
    /// If no timer exists, this method does nothing.
    pub fn reset_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.reset();
            timer.start();
        }
    }

    /// Updates the timer countdown and returns whether the timer is still running.
    ///
    /// # Returns
    ///
    /// `true` if the timer is running after the update, `false` otherwise.
    /// If no timer exists, returns `false`.
    pub fn update_timer(&mut self) -> bool {
        if let Some(timer) = &mut self.timer {
            timer.update()
        } else {
            false
        }
    }

    /// Checks if the timer has expired (reached zero).
    ///
    /// # Returns
    ///
    /// `true` if the timer exists and has expired, `false` otherwise.
    pub fn is_timer_expired(&self) -> bool {
        self.timer.as_ref().map(|t| t.is_expired()).unwrap_or(false)
    }

    /// Gets the formatted timer text for display.
    ///
    /// # Returns
    ///
    /// A string representing the current timer value in "MM.SS" or "00.00" if no timer exists.
    pub fn get_timer_text(&self) -> String {
        self.timer
            .as_ref()
            .map_or("00.00".to_string(), |t| t.format_time())
    }

    /// Gets the current color of the timer based on its state.
    ///
    /// # Returns
    ///
    /// The color representing the timer's current state (normal, warning, or critical).
    /// Returns white if no timer exists.
    pub fn get_timer_color(&self) -> Color {
        self.timer
            .as_ref()
            .map_or(Color::rgb(255, 255, 255), |t| t.get_current_color())
    }

    /// Sets the current game level.
    ///
    /// # Parameters
    ///
    /// * `level` - The new level number to set.
    pub fn set_level(&mut self, level: i32) {
        self.level = level;
    }

    /// Gets the current game level.
    ///
    /// # Returns
    ///
    /// The current level number.
    pub fn get_level(&self) -> i32 {
        self.level
    }

    /// Gets the formatted level text for display.
    ///
    /// # Returns
    ///
    /// A string in the format "Level: X" where X is the current level.
    pub fn get_level_text(&self) -> String {
        format!("Level: {}", self.level)
    }

    /// Sets the current game score.
    ///
    /// # Parameters
    ///
    /// * `score` - The new score value to set.
    pub fn set_score(&mut self, score: u32) {
        self.score = score;
    }

    /// Gets the current game score.
    ///
    /// # Returns
    ///
    /// The current score value.
    pub fn get_score(&self) -> u32 {
        self.score
    }

    /// Gets the formatted score text for display.
    ///
    /// # Returns
    ///
    /// A string in the format "Score: X" where X is the current score.
    pub fn get_score_text(&self) -> String {
        format!("Score: {}", self.score)
    }

    /// Pauses the timer if it is currently running.
    ///
    /// If no timer exists, this method does nothing.
    pub fn pause_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.pause();
        }
    }

    /// Resumes the timer if it is currently paused.
    ///
    /// If no timer exists, this method does nothing.
    pub fn resume_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.resume();
        }
    }
}

/// Sets up the timer, score, and level display using the TextRenderer
pub fn initialize_game_ui(
    text_renderer: &mut TextRenderer,
    game_ui: &GameUIManager,
    window: &Window,
) {
    let size = window.inner_size();
    let width = size.width;
    let height = size.height;

    // --- Responsive scaling logic ---
    // If the window is large, scale up the text; otherwise, use default sizes
    let (timer_font_size, timer_line_height, timer_max_width, timer_max_height) = if width >= 1920 {
        (80.0, 100.0, 300.0, 120.0)
    } else if width >= 1600 || height >= 900 {
        (60.0, 76.0, 200.0, 80.0)
    } else {
        (48.0, 60.0, 150.0, 60.0)
    };
    let (label_font_size, label_line_height, label_max_width, label_max_height) =
        if width >= 1600 || height >= 900 {
            (24.0, 28.0, 160.0, 32.0)
        } else {
            (18.0, 22.0, 120.0, 25.0)
        };

    // Timer display (decimal-aligned at top)
    let timer_text = game_ui.get_timer_text();
    let timer_style = TextStyle {
        font_family: "HankenGrotesk".to_string(),
        font_size: timer_font_size,
        line_height: timer_line_height,
        color: Color::rgb(100, 255, 100),
        weight: glyphon::Weight::BOLD,
        style: glyphon::Style::Normal,
    };
    // Find decimal position in timer_text
    let decimal_index = timer_text.find('.').unwrap_or(timer_text.len() - 1) + 1;
    let decimal_substr = &timer_text[..decimal_index];
    let (_min_x, decimal_offset, _h) = text_renderer.measure_text(decimal_substr, &timer_style);
    let timer_position = TextPosition {
        x: (width as f32 / 2.0) - decimal_offset,
        y: 10.0,
        max_width: Some(timer_max_width),
        max_height: Some(timer_max_height),
    };
    text_renderer.create_text_buffer(
        "main_timer",
        &timer_text,
        Some(timer_style),
        Some(timer_position),
    );

    // Level display (top left, above score)
    let level_style = TextStyle {
        font_family: "HankenGrotesk".to_string(),
        font_size: label_font_size,
        line_height: label_line_height,
        color: Color::rgb(255, 255, 150),
        weight: glyphon::Weight::NORMAL,
        style: glyphon::Style::Normal,
    };
    let level_position = TextPosition {
        x: 20.0,
        y: 20.0,
        max_width: Some(label_max_width),
        max_height: Some(label_max_height),
    };
    text_renderer.create_text_buffer(
        "level",
        &game_ui.get_level_text(),
        Some(level_style),
        Some(level_position),
    );

    // Score display (top left, below level, left edge aligned)
    let score_style = TextStyle {
        font_family: "HankenGrotesk".to_string(),
        font_size: label_font_size,
        line_height: label_line_height,
        color: Color::rgb(150, 255, 255),
        weight: glyphon::Weight::NORMAL,
        style: glyphon::Style::Normal,
    };
    let score_position = TextPosition {
        x: 20.0,
        y: 50.0,
        max_width: Some(label_max_width),
        max_height: Some(label_max_height),
    };
    text_renderer.create_text_buffer(
        "score",
        &game_ui.get_score_text(),
        Some(score_style),
        Some(score_position),
    );
}

/// Helper to update the text content of a buffer and re-apply style
fn update_text_content(
    text_renderer: &mut TextRenderer,
    id: &str,
    new_text: &str,
) -> Result<(), String> {
    if let Some(buffer) = text_renderer.text_buffers.get_mut(id) {
        buffer.text_content = new_text.to_string();
        // Re-apply style to update the buffer
        let style = buffer.style.clone();
        text_renderer.update_style(id, style)
    } else {
        Err(format!("Text buffer '{}' not found", id))
    }
}

/// Call this every frame to update the timer, score, and level displays
pub fn update_game_ui(
    text_renderer: &mut TextRenderer,
    game_ui: &mut GameUIManager,
    current_screen: &CurrentScreen,
    window: &Window,
) -> bool {
    // Only update the timer if in Game
    let timer_expired = if let CurrentScreen::Game = current_screen {
        game_ui.update_timer()
    } else {
        false
    };

    // Update timer display
    let timer_text = game_ui.get_timer_text();
    let _ = update_text_content(text_renderer, "main_timer", &timer_text);
    // Update timer color by updating style
    if let Some(buffer) = text_renderer.text_buffers.get_mut("main_timer") {
        let mut style = buffer.style.clone();
        style.color = game_ui.get_timer_color();
        let _ = text_renderer.update_style("main_timer", style);
    }

    // Update level and score displays
    let _ = update_text_content(text_renderer, "level", &game_ui.get_level_text());
    let _ = update_text_content(text_renderer, "score", &game_ui.get_score_text());

    // Adjust timer position if window size changes
    let size = window.inner_size();
    let width = size.width;
    let height = size.height;
    let (timer_max_width, timer_max_height) = if width >= 1920 {
        (300.0, 120.0)
    } else if width >= 1600 || height >= 900 {
        (200.0, 80.0)
    } else {
        (150.0, 60.0)
    };
    // Align decimal point with center
    let timer_style = if let Some(buffer) = text_renderer.text_buffers.get("main_timer") {
        buffer.style.clone()
    } else {
        TextStyle::default()
    };
    let decimal_index = timer_text.find('.').unwrap_or(timer_text.len() - 1) + 1;
    let decimal_substr = &timer_text[..decimal_index];
    let (_min_x, decimal_offset, _h) = text_renderer.measure_text(decimal_substr, &timer_style);
    let timer_position = TextPosition {
        x: (width as f32 / 2.0) - decimal_offset,
        y: 10.0,
        max_width: Some(timer_max_width),
        max_height: Some(timer_max_height),
    };
    if let Some(buffer) = text_renderer.text_buffers.get_mut("main_timer") {
        buffer.position = timer_position;
    }

    timer_expired
}
