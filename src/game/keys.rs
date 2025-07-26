//! Keyboard and mouse input handling for the game.
//!
//! This module defines the [`GameKey`] enum for abstracting game actions from physical keys,
//! and provides [`KeyState`] for tracking pressed keys and updating the [`GameState`] accordingly.
//! It also includes utilities for mapping from winit key events to game actions.

use crate::game::{CurrentScreen, GameState};
use std::collections::HashSet;
use winit::keyboard;
/// Enum representing all possible in-game actions that can be triggered by keyboard or mouse input.
///
/// This abstraction allows the game logic to be decoupled from specific physical keys or buttons.
/// Variants include movement, mouse buttons, toggles, and quitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    /// Left mouse button.
    MouseButtonLeft,
    /// Right mouse button.
    MouseButtonRight,
    /// Move player forward (W or Up Arrow).
    MoveForward,
    /// Move player backward (S or Down Arrow).
    MoveBackward,
    /// Move player left (A or Left Arrow).
    MoveLeft,
    /// Move player right (D or Right Arrow).
    MoveRight,
    /// Sprint (Shift).
    Sprint,
    /// Jump (Space).
    Jump,
    /// Toggle UI sliders (C).
    ToggleSliders,
    /// Quit the game (`).
    Quit,
    /// Escape key (toggle mouse capture).
    Escape,
    /// Toggle Bounding Boxes (B).
    ToggleBoundingBoxes,
    /// Toggle Upgrade Menu (U).
    ToggleUpgradeMenu,
}

/// Tracks the set of currently pressed game keys.
///
/// Use [`press_key`] and [`release_key`] to update the state, and [`is_pressed`] to query.
/// The [`update`] method applies the current key state to the [`GameState`].
#[derive(Debug, Default)]
pub struct KeyState {
    /// Set of currently pressed keys.
    pub pressed_keys: HashSet<GameKey>,
}

impl KeyState {
    /// Creates a new, empty [`KeyState`]
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    /// Marks a key as pressed.
    pub fn press_key(&mut self, key: GameKey) {
        self.pressed_keys.insert(key);
    }

    /// Marks a key as released.
    pub fn release_key(&mut self, key: GameKey) {
        self.pressed_keys.remove(&key);
    }

    /// Checks if a key is currently pressed.
    pub fn is_pressed(&self, key: GameKey) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn _clear(&mut self) {
        self.pressed_keys.clear();
    }

    /// Updates the [`GameState`] based on the current pressed keys.
    ///
    /// - Handles movement, sprinting, jumping, toggling UI, and mouse capture.
    /// - Adjusts player speed for sprinting.
    /// - Moves the player according to pressed movement keys.
    /// - Handles mouse and escape key actions.
    pub fn update(&mut self, game_state: &mut GameState) {
        // Handle sprint speed changes
        let is_sprinting = self.is_pressed(GameKey::Sprint) && game_state.player.stamina > 0.0;
        let forward = self.is_pressed(GameKey::MoveForward);
        let backward = self.is_pressed(GameKey::MoveBackward);
        let left = self.is_pressed(GameKey::MoveLeft);
        let right = self.is_pressed(GameKey::MoveRight);
        let is_moving = forward || backward || left || right;
        // Update stamina
        game_state
            .player
            .update_stamina(is_sprinting, is_moving, game_state.delta_time);
        if is_sprinting {
            game_state.player.speed = game_state.player.base_speed * 1.75;
        } else {
            game_state.player.speed = game_state.player.base_speed;
        }

        if game_state.current_screen != CurrentScreen::Game {
            game_state
                .audio_manager
                .stop_movement()
                .expect("Failed to start sprinting sound");
        }

        if game_state.current_screen == CurrentScreen::Game
            || game_state.current_screen == CurrentScreen::ExitReached
        {
            // Handle movement audio based on current state
            if is_moving {
                if is_sprinting {
                    // Switch to sprint audio if not already sprinting
                    if !game_state.audio_manager.is_sprinting() {
                        game_state
                            .audio_manager
                            .start_sprinting()
                            .expect("Failed to start sprinting sound");
                    }
                } else {
                    // Switch to walking audio if not already walking
                    if !game_state.audio_manager.is_walking() {
                        game_state
                            .audio_manager
                            .start_walking()
                            .expect("Failed to start walking sound");
                    }
                }
            } else {
                // Stop movement sounds when player is not moving
                if game_state.audio_manager.is_moving() {
                    game_state
                        .audio_manager
                        .stop_movement()
                        .expect("Failed to stop movement sound");
                }
            }

            // Handle player movement with collision
            game_state.player.move_with_collision(
                &mut game_state.audio_manager,
                &game_state.collision_system,
                game_state.delta_time,
                forward,
                backward,
                left,
                right,
            );
        }

        // Handle non-movement keys
        if self.is_pressed(GameKey::MouseButtonLeft) && game_state.capture_mouse {
            if game_state.current_screen == CurrentScreen::Loading {
                if game_state.maze_path.is_some() {
                    game_state.current_screen = CurrentScreen::Game;
                    if let Some(timer) = &mut game_state.game_ui.timer {
                        timer.start();
                    }
                }
            } else if game_state.current_screen == CurrentScreen::GameOver {
                game_state.current_screen = CurrentScreen::NewGame;
            }
        }
    }
}

macro_rules! match_char_key {
    ($c:expr, {
        $($key:literal => $variant:expr),* $(,)?
    }) => {{
        match $c.to_ascii_lowercase().as_str() {
            $($key => Some($variant),)*
            _ => None,
        }
    }};
}

macro_rules! match_named_key {
    ($k:expr, {
        $($key:ident => $variant:expr),* $(,)?
    }) => {{
        match $k {
            $(winit::keyboard::NamedKey::$key => Some($variant),)*
            _ => None,
        }
    }};
}

/// Converts a winit [`keyboard::Key`] to a [`GameKey`] if it matches a mapped action.
///
/// Supports both named keys (arrows, shift, space, escape) and character keys (WASD, C, Q).
///
/// # Arguments
/// * `key` - The winit key event to convert.
///
/// # Returns
/// * `Some(GameKey)` if the key maps to a game action.
/// * `None` otherwise.
pub fn winit_key_to_game_key(key: &keyboard::Key) -> Option<GameKey> {
    match key {
        keyboard::Key::Named(named) => match_named_key!(named, {
            ArrowUp => GameKey::MoveForward,
            ArrowDown => GameKey::MoveBackward,
            ArrowLeft => GameKey::MoveLeft,
            ArrowRight => GameKey::MoveRight,
            Shift => GameKey::Sprint,
            Space => GameKey::Jump,
            Escape => GameKey::Escape,
        }),

        keyboard::Key::Character(c) => match_char_key!(c, {
            "w" => GameKey::MoveForward,
            "s" => GameKey::MoveBackward,
            "a" => GameKey::MoveLeft,
            "d" => GameKey::MoveRight,
            "c" => GameKey::ToggleSliders,
            "`" => GameKey::Quit,
            "b" => GameKey::ToggleBoundingBoxes,
            "u" => GameKey::ToggleUpgradeMenu,
        }),

        _ => None,
    }
}
