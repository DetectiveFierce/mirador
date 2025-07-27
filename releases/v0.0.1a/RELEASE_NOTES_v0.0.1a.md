# Mirador v0.0.1-alpha Release Notes

## Release Information
- **Version**: 0.0.1-alpha
- **Release Date**: $(date)
- **Platform**: Linux (x86_64), Windows (x86_64)

## What's New
- Initial alpha release of Mirador
- Maze exploration game with audio support
- WGPU-based rendering engine
- Compass navigation system
- Enemy AI and collision detection
- Upgrade system for player progression

## Installation

### Linux
1. Download `Mirador-v0.0.1a-Linux`
2. Make the file executable: `chmod +x Mirador-v0.0.1a-Linux`
3. Run the game: `./Mirador-v0.0.1a-Linux`

### Windows
1. Download `Mirador-v0.0.1a-Windows.exe`
2. Double-click the executable to run the game

## System Requirements

### Linux
- Linux x86_64
- OpenGL 3.3+ or Vulkan 1.0+
- Audio support (ALSA/PulseAudio)

### Windows
- Windows 10/11 (x86_64)
- DirectX 11+ or Vulkan 1.0+
- Audio support (DirectSound/WASAPI)

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
- Additional platform support planned (macOS, etc.)
- Performance optimizations and bug fixes
- Enhanced audio and graphics features 