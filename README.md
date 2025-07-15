# Mirador

**Mirador** is a modular, interactive 3D maze game written in Rust, focused on real-time rendering, procedural maze generation, and immersive gameplay. It leverages modern GPU technologies and immediate-mode GUI frameworks to provide a responsive and visually engaging experience.

---

## Purpose

Mirador is a work in progress 3D maze game engine written entirely in Rust. It is designed to be modular, real-time, and memory-efficient. It achieves this by employing several rendering pipelines that each handle specific aspects of the game's visual and interactive elements, all of which leverage the computational power of the GPU via the WGPU graphics API. The project currently supports:

- **Real-time 3D rendering** using the WGPU graphics API with advanced shaders
- **Procedural maze generation** using Kruskal's algorithm with animated generation
- **3D collision detection** with BVH (Bounding Volume Hierarchy) for efficient physics
- **Spatial audio system** with 3D positional sound effects and music
- **Enemy AI** with pathfinding and level-based aggression scaling
- **Scoring system** with time-based bonuses and level progression
- **Game state management** with multiple screens (Loading, Game, Pause, Game Over)
- **Integration with egui** for overlays, controls, and interactive panels
- **Responsive input handling** with WASD movement, mouse look, and sprint mechanics

---

## Game Features

### Core Gameplay
- **3D First-Person Navigation**: Smooth movement with WASD keys and mouse look
- **Procedural Mazes**: Each level generates a unique 25x25 maze using Kruskal's algorithm
- **Time-Based Challenges**: Complete levels before time runs out with dynamic difficulty
- **Scoring System**: Performance-based scoring with speed bonuses and level progression
- **Enemy Pursuit**: AI enemies that hunt the player with increasing aggression per level

### Audio Experience
- **3D Spatial Audio**: Positional sound effects using the Kira audio engine
- **Dynamic Footsteps**: Walking and sprinting sounds that respond to movement
- **Environmental Audio**: Wall collision sounds and completion effects
- **Enemy Audio**: Spatial enemy sounds that follow AI movement

### Visual Features
- **Real-time Shaders**: Advanced fragment shaders with noise-based effects
- **Animated Loading Screen**: Live maze generation visualization
- **3D Graphics**: Full 3D environment with walls, floor, and exit portal effects
- **UI Overlays**: Real-time timer, score, and level display

---

## Architecture Overview

Mirador is organized into several core modules:

- **[App](/src/app.rs)**: The main application object, responsible for initialization, event handling, rendering, and orchestrating the game state.
- **[Game](/src/game/)**: Contains logic for player state, input handling, collision detection, enemy AI, audio management, and core gameplay mechanics.
- **[Maze](/src/maze/)**: Handles procedural maze generation using Kruskal's algorithm, storage, and rendering data.
- **[Renderer](/src/renderer/)**: Manages all rendering pipelines including game scene, loading screen, and UI overlays.
- **[UI](/src/ui/)**: Implements egui-based overlays for development, debugging, and game interface.
- **[Math](/src/math/)**: Provides vector and matrix utilities for graphics and game logic.

The application initializes a WGPU instance and event loop, sets up rendering and UI pipelines, and manages all state transitions and user interactions through a central `App` struct.

---

## Technologies Used

- **[Rust](https://www.rust-lang.org/)** (edition 2024): Systems programming language for performance and safety
- **[WGPU](https://wgpu.rs)**: Modern, portable graphics API for GPU rendering
- **[egui](https://github.com/emilk/egui)**: Immediate-mode GUI library for Rust
- **[winit](https://github.com/rust-windowing/winit)**: Cross-platform window and event loop management
- **[Kira](https://github.com/tesselode/kira)**: Spatial audio engine for 3D sound effects
- **[rand](https://github.com/rust-random/rand)**: Random number generation for procedural content
- **[chrono](https://github.com/chronotope/chrono)**: Time and date utilities
- **[glyphon](https://github.com/grovesNL/glyphon)**: Modern Rust API for rendering text on the GPU via wgpu

---

## Getting Started

Mirador is intended for developers and enthusiasts interested in graphics programming, game development, or Rust-based application architecture. To explore or contribute:

### Prerequisites
- Rust toolchain (latest stable version)
- Graphics drivers that support Vulkan, DirectX 12, or Metal
- Audio drivers for spatial audio support

### Building and Running
```bash
# Clone the repository
git clone <repository-url>
cd mirador

# Build the project
cargo build

# Run the game
cargo run

# For development with heap profiling (optional)
cargo run --features dhat-heap
```

### Controls
- **WASD**: Move forward/backward/left/right
- **Mouse**: Look around (camera control)
- **Shift**: Sprint (increased movement speed)
- **Left Click**: Interact with UI elements
- **Escape**: Toggle mouse capture
- **Q**: Quit game

---

## Development

### Project Structure
```
mirador/
├── src/
│   ├── app.rs              # Main application logic
│   ├── game/               # Game mechanics and state
│   │   ├── audio.rs        # Spatial audio system
│   │   ├── collision.rs    # 3D collision detection
│   │   ├── enemy.rs        # Enemy AI and pathfinding
│   │   ├── keys.rs         # Input handling
│   │   ├── player.rs       # Player state and movement
│   │   └── mod.rs          # Game state management
│   ├── maze/               # Maze generation and parsing
│   ├── renderer/           # Graphics and rendering
│   ├── ui/                 # User interface
│   └── math/               # Mathematical utilities
├── assets/                 # Game assets (audio, images)
├── fonts/                  # Typography resources
└── Cargo.toml             # Project dependencies
```

### Key Features Implementation
- **Collision System**: BVH-based collision detection with wall sliding
- **Audio Engine**: 3D spatial audio with distance-based effects
- **Enemy AI**: Pathfinding with level-based aggression scaling
- **Maze Generation**: Kruskal's algorithm with animated visualization
- **Scoring**: Time-based performance metrics with bonuses

---

## Documentation

For the development blog see:
- **[finitesample.space](https://https://finitesample.space/about/)**

For detailed in-code documentation, please refer to:
- **[Rust Doc Documentation](https://DetectiveFierce.github.io/mirador/mirador/index.html)**

---

## License

Mirador is released under an open-source license. See the repository for details.

---

## Contributing

Contributions are welcome! The project is actively developed and open to improvements in:
- Graphics and rendering optimizations
- Game mechanics and balance
- Audio system enhancements
- Performance improvements
- Bug fixes and code quality

Please ensure all code follows Rust conventions and includes appropriate documentation.
