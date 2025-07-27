# Mirador v0.0.1-alpha Release Notes

## Release Information
- **Version**: 0.0.1-alpha
- **Release Date**: $(date)
- **Platform**: Linux (x86_64)

## What's New
- Initial alpha release of Mirador
- Maze exploration game with audio support
- WGPU-based rendering engine
- Compass navigation system
- Enemy AI and collision detection
- Upgrade system for player progression

## Installation
1. Download the appropriate binary for your platform
2. Make the file executable: `chmod +x Mirador-v0.0.1a-Linux`
3. Run the game: `./Mirador-v0.0.1a-Linux`

## System Requirements
- Linux x86_64
- OpenGL 3.3+ or Vulkan 1.0+
- Audio support (ALSA/PulseAudio)

## Known Issues
- This is an alpha release and may contain bugs
- Performance may vary depending on hardware
- Some audio features may not work on all systems

## Building from Source
```bash
git clone <repository-url>
cd mirador
cargo build --release
```

## Next Steps
- Windows executable will be added in a future release
- Additional platform support planned
- Performance optimizations and bug fixes 