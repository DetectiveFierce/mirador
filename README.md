# Mirador

A 3D maze game where you navigate procedurally generated labyrinths while being hunted by increasingly aggressive enemies. Built in Rust with real-time GPU rendering and spatial audio.

<img width="2555" height="1038" alt="image" src="https://github.com/user-attachments/assets/a6611345-99eb-4a43-a5bf-6b24a26e6a0f" />

---

## What is Mirador?

Mirador is a first-person maze exploration game that combines procedural generation, real-time 3D graphics, and spatial audio. Each level generates a unique 25x25 maze using Kruskal's algorithm, complete with animated generation visualization.

### Core Experience
- **Navigate** through procedurally generated 3D mazes
- **Escape** from enemies that become more aggressive each level
- **Race** against time to complete levels and earn bonuses
- **Immerse** yourself in spatial audio that responds to your movement
- **Progress** through an upgrade system that enhances your abilities

### Technical Highlights
- **WGPU-powered rendering** with advanced fragment shaders
- **3D spatial audio** using the Kira engine
- **Real-time collision detection** with BVH optimization
- **GPU-accelerated text rendering** via glyphon
- **Cross-platform** support (Linux, Windows)
- **Progression system** with 6 unique upgrades and rarity tiers

---

## Quick Start

### Play Now
Download pre-built binaries for immediate play:
- **[Linux (x86_64)](https://github.com/DetectiveFierce/mirador/releases/tag/v0.0.1a)** - `Mirador-v0.0.1a-Linux`
- **[Windows (x86_64)](https://github.com/DetectiveFierce/mirador/releases/tag/v0.0.1a)** - `Mirador-v0.0.1a-Windows.exe`

### Build from Source
```bash
git clone <repository-url>
cd mirador
cargo build --release
cargo run --release
```

### Controls
- **WASD** - Move
- **Mouse** - Look around
- **Shift** - Sprint
- **Escape** - Toggle mouse capture
- **Q** - Quit

---

## Game Features

### Maze Generation
Each level creates a unique 25x25 maze using Kruskal's algorithm. Watch the walls form in real-time during the loading screen.

### Enemy AI
Enemies hunt you with pathfinding algorithms. Their aggression scales with each level, creating increasing tension.

### Upgrade System
Every 3 levels, choose from 3 randomly selected upgrades to enhance your abilities:

**Common Upgrades (40% chance)**
- **Speed Up** - Increases movement and sprint speed by 10% per level

**Uncommon Upgrades (30% chance)**
- **Slow Time** - Adds 5 seconds to the level timer per level
- **Tall Boots** - Increases height by 3 units per level for better visibility

**Rare Upgrades (20% chance)**
- **Silent Step** - Reduces noise and improves stealth
- **Head Start** - Delays enemy movement at level start

**Epic Upgrades (8% chance)**
- **Dash** - Increases maximum stamina by 10% per level

### Audio System
- **3D spatial audio** - sounds come from their actual locations
- **Dynamic footsteps** - walking and sprinting sounds
- **Environmental feedback** - wall collisions and completion effects
- **Enemy audio** - spatial sounds that follow AI movement

### Visual Effects
- **Real-time shaders** with noise-based effects
- **Animated loading screen** showing maze generation
- **3D environment** with walls, floor, and exit portal effects
- **UI overlays** with real-time timer and score display

### Cross-Platform Support
- **Linux** - Native support with desktop integration
- **Windows** - DirectX and Vulkan rendering
- **Automatic builds** for both platforms
- **Platform-specific optimizations** for performance

---

## System Requirements

### Linux
- Linux x86_64
- OpenGL 3.3+ or Vulkan 1.0+
- Audio support (ALSA/PulseAudio)

### Windows
- Windows 10/11 (x86_64)
- DirectX 11+ or Vulkan 1.0+
- Audio support (DirectSound/WASAPI)

---

## Architecture

Mirador uses a modular architecture with specialized components:

- **[App](/src/app/)** - Main application orchestrator
- **[Game](/src/game/)** - Core gameplay mechanics and state
- **[Maze](/src/maze/)** - Procedural generation using Kruskal's algorithm
- **[Renderer](/src/renderer/)** - WGPU-based rendering pipelines
- **[UI](/src/ui/)** - Interface elements and menus
- **[Math](/src/math/)** - Vector and matrix utilities

The game leverages GPU compute for both graphics and text rendering, ensuring smooth performance across different hardware configurations.

---

## Technologies

- **[Rust](https://www.rust-lang.org/)** (2024 edition) - Systems programming with memory safety
- **[WGPU](https://wgpu.rs)** - Modern, portable graphics API
- **[Kira](https://github.com/tesselode/kira)** - Spatial audio engine
- **[Glyphon](https://github.com/grovesNL/glyphon)** - GPU-accelerated text rendering
- **[Winit](https://github.com/rust-windowing/winit)** - Cross-platform window management

---

## Development

### Project Structure
```
mirador/
├── src/
│   ├── app/                # Application lifecycle
│   ├── game/               # Game mechanics
│   │   ├── audio.rs        # Spatial audio
│   │   ├── collision.rs    # 3D collision detection
│   │   ├── enemy.rs        # AI pathfinding
│   │   ├── player.rs       # Player state
│   │   └── upgrades.rs     # Upgrade system
│   ├── maze/               # Procedural generation
│   ├── renderer/           # Graphics pipelines
│   └── ui/                 # Interface elements
```

### Key Implementations
- **Collision System** - BVH-based detection with wall sliding
- **Audio Engine** - 3D spatial audio with distance effects
- **Enemy AI** - Pathfinding with level-based aggression
- **Maze Generation** - Kruskal's algorithm with visualization
- **Text Rendering** - Thread-safe GPU-accelerated rendering
- **Upgrade System** - Weighted random selection with rarity tiers

---

## Documentation

- **[Development Blog](https://finitesample.space/about/)**
- **[API Documentation](https://DetectiveFierce.github.io/mirador/mirador/index.html)**

---

## Contributing

Contributions welcome! Areas of interest:
- Graphics and rendering optimizations
- Game mechanics and balance
- Audio system enhancements
- Performance improvements
- Bug fixes and code quality

Please follow Rust conventions and include appropriate documentation or don't i'm not really that picky.

---

## Known Issues

- Pre-Alpha release - may contain bugs (please report them!)
- Performance varies by hardware
- Debug builds have slow startup (release builds are fast)
- Some audio features may not work on all systems

---

## License

Open source, not licensed as of right now 
