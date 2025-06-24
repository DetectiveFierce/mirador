use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    ToggleSliders,
    Quit,
}

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
