use std::collections::HashSet;
use winit::keyboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Sprint,
    Jump,
    ToggleSliders,
    Quit,
}

#[derive(Debug)]
pub struct KeyState {
    pressed_keys: HashSet<GameKey>,
}

impl KeyState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    pub fn press_key(&mut self, key: GameKey) {
        self.pressed_keys.insert(key);
    }

    pub fn release_key(&mut self, key: GameKey) {
        self.pressed_keys.remove(&key);
    }

    pub fn is_pressed(&self, key: GameKey) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn _clear(&mut self) {
        self.pressed_keys.clear();
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

// Convert winit key to our game key enum
pub fn winit_key_to_game_key(key: &keyboard::Key) -> Option<GameKey> {
    match key {
        keyboard::Key::Named(named) => match_named_key!(named, {
            ArrowUp => GameKey::MoveForward,
            ArrowDown => GameKey::MoveBackward,
            ArrowLeft => GameKey::MoveLeft,
            ArrowRight => GameKey::MoveRight,
            Shift => GameKey::Sprint,
            Space => GameKey::Jump,
        }),

        keyboard::Key::Character(c) => match_char_key!(c, {
            "w" => GameKey::MoveForward,
            "s" => GameKey::MoveBackward,
            "a" => GameKey::MoveLeft,
            "d" => GameKey::MoveRight,
            "c" => GameKey::ToggleSliders,
            "q" => GameKey::Quit,
        }),

        _ => None,
    }
}
