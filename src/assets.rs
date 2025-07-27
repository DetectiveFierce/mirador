//! # Assets Module
//!
//! This module contains all game assets embedded in the binary using `include_bytes!()`.
//! This ensures that all assets are available at runtime without requiring external files.

// Font assets
/// Hanken Grotesk Regular font data
pub const HANKEN_GROTESK_REGULAR: &[u8] =
    include_bytes!("../fonts/HankenGrotesk/HankenGrotesk-Regular.ttf");
/// Hanken Grotesk Medium font data
pub const HANKEN_GROTESK_MEDIUM: &[u8] =
    include_bytes!("../fonts/HankenGrotesk/HankenGrotesk-Medium.ttf");
/// Hanken Grotesk Bold font data
pub const HANKEN_GROTESK_BOLD: &[u8] =
    include_bytes!("../fonts/HankenGrotesk/HankenGrotesk-Bold.ttf");

// Image assets
/// Game title image data
pub const TITLE_IMAGE: &[u8] = include_bytes!("../assets/Mirador-title.png");
/// Slime enemy image data
pub const SLIME_IMAGE: &[u8] = include_bytes!("../assets/Slime.png");
/// Frankie character image data
pub const FRANKIE_IMAGE: &[u8] = include_bytes!("../assets/frankie.png");
/// Jeffree character image data
pub const JEFFREE_IMAGE: &[u8] = include_bytes!("../assets/jeffree.png");
/// Maze icon image data
pub const MAZE_ICON_IMAGE: &[u8] = include_bytes!("../assets/maze-icon.png");
/// Tiles texture image data
pub const TILES_IMAGE: &[u8] = include_bytes!("../assets/tiles.jpg");

// Compass assets
/// Compass base image data
pub const COMPASS_BASE: &[u8] = include_bytes!("../assets/compass/compass.png");
/// Gold compass image data
pub const GOLD_COMPASS: &[u8] = include_bytes!("../assets/compass/gold-compass.png");

// Compass needle assets
/// Compass needle position 0 image data
pub const NEEDLE_0: &[u8] = include_bytes!("../assets/compass/needle/needle-0.png");
/// Compass needle position 1 image data
pub const NEEDLE_1: &[u8] = include_bytes!("../assets/compass/needle/needle-1.png");
/// Compass needle position 2 image data
pub const NEEDLE_2: &[u8] = include_bytes!("../assets/compass/needle/needle-2.png");
/// Compass needle position 3 image data
pub const NEEDLE_3: &[u8] = include_bytes!("../assets/compass/needle/needle-3.png");
/// Compass needle position 4 image data
pub const NEEDLE_4: &[u8] = include_bytes!("../assets/compass/needle/needle-4.png");
/// Compass needle position 5 image data
pub const NEEDLE_5: &[u8] = include_bytes!("../assets/compass/needle/needle-5.png");
/// Compass needle position 6 image data
pub const NEEDLE_6: &[u8] = include_bytes!("../assets/compass/needle/needle-6.png");
/// Compass needle position 7 image data
pub const NEEDLE_7: &[u8] = include_bytes!("../assets/compass/needle/needle-7.png");
/// Compass needle position 8 image data
pub const NEEDLE_8: &[u8] = include_bytes!("../assets/compass/needle/needle-8.png");
/// Compass needle position 9 image data
pub const NEEDLE_9: &[u8] = include_bytes!("../assets/compass/needle/needle-9.png");
/// Compass needle position 10 image data
pub const NEEDLE_10: &[u8] = include_bytes!("../assets/compass/needle/needle-10.png");
/// Compass needle position 11 image data
pub const NEEDLE_11: &[u8] = include_bytes!("../assets/compass/needle/needle-11.png");

// Icon assets
/// Blank icon image data
pub const BLANK_ICON: &[u8] = include_bytes!("../assets/icons/blank-icon.png");
/// Dash ability icon image data
pub const DASH_ICON: &[u8] = include_bytes!("../assets/icons/dash-icon.png");
/// Head start ability icon image data
pub const HEAD_START_ICON: &[u8] = include_bytes!("../assets/icons/head-start-icon.png");
/// Silent step ability icon image data
pub const SILENT_STEP_ICON: &[u8] = include_bytes!("../assets/icons/silent-step-icon.png");
/// Slower seconds ability icon image data
pub const SLOWER_SECONDS_ICON: &[u8] = include_bytes!("../assets/icons/slower-seconds-icon.png");
/// Speed up ability icon image data
pub const SPEED_UP_ICON: &[u8] = include_bytes!("../assets/icons/speed-up-icon.png");
/// Tall boots ability icon image data
pub const TALL_BOOTS_ICON: &[u8] = include_bytes!("../assets/icons/tall-boots-icon.png");
/// Unknown ability icon image data
pub const UNKNOWN_ICON: &[u8] = include_bytes!("../assets/icons/unknown-icon.png");

// Audio assets
/// Beeper rise sound effect data
pub const AUDIO_BEEPER_RISE: &[u8] = include_bytes!("../assets/audio/beeper-rise.ogg");
/// Level complete sound effect data
pub const AUDIO_COMPLETE: &[u8] = include_bytes!("../assets/audio/complete.wav");
/// Jeffree Star ASMR sound effect data
pub const AUDIO_JEFFREE_STAR_ASMR: &[u8] = include_bytes!("../assets/audio/jeffree-star-asmr.ogg");
/// Menu select sound effect data
pub const AUDIO_SELECT: &[u8] = include_bytes!("../assets/audio/select.ogg");
/// Single step sound effect data
pub const AUDIO_SINGLE_STEP: &[u8] = include_bytes!("../assets/audio/single_step.wav");
/// Slime track sound effect data
pub const AUDIO_SLIME_TRACK: &[u8] = include_bytes!("../assets/audio/slime-track.ogg");
/// Upgrade sound effect data
pub const AUDIO_UPGRADE: &[u8] = include_bytes!("../assets/audio/upgrade.ogg");
/// Wall hit sound effect data
pub const AUDIO_WALL: &[u8] = include_bytes!("../assets/audio/wall.wav");

// Music assets
/// Main game music track data
pub const MUSIC_MAIN_TRACK: &[u8] = include_bytes!("../assets/audio/music/Mirador Main Track.ogg");
/// Stripped main game music track data
pub const MUSIC_MAIN_TRACK_STRIPPED: &[u8] =
    include_bytes!("../assets/audio/music/Mirador Main Track - Stripped.ogg");

/// Returns all compass needle textures in order
pub fn compass_needles() -> &'static [&'static [u8]] {
    &[
        NEEDLE_0, NEEDLE_1, NEEDLE_2, NEEDLE_3, NEEDLE_4, NEEDLE_5, NEEDLE_6, NEEDLE_7, NEEDLE_8,
        NEEDLE_9, NEEDLE_10, NEEDLE_11,
    ]
}

/// Returns all icon textures with their IDs
pub fn icon_textures() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("blank_icon", BLANK_ICON),
        ("dash_icon", DASH_ICON),
        ("head_start_icon", HEAD_START_ICON),
        ("silent_step_icon", SILENT_STEP_ICON),
        ("slower_seconds_icon", SLOWER_SECONDS_ICON),
        ("speed_up_icon", SPEED_UP_ICON),
        ("tall_boots_icon", TALL_BOOTS_ICON),
        ("unknown_icon", UNKNOWN_ICON),
    ]
}

/// Returns all font data with their names
pub fn fonts() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("Hanken Grotesk", HANKEN_GROTESK_REGULAR),
        ("Hanken Grotesk Medium", HANKEN_GROTESK_MEDIUM),
        ("Hanken Grotesk Bold", HANKEN_GROTESK_BOLD),
    ]
}
