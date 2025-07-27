# Mirador Performance Benchmarks

This document describes the comprehensive performance benchmarking system added to the Mirador game engine to identify and measure the most taxing parts of the game.

## Overview

The benchmarking system provides detailed timing measurements for critical game engine operations, helping identify performance bottlenecks and optimize the most time-consuming parts of the game.

## Benchmark Categories

### 1. Frame-Level Benchmarks

#### `total_frame`
- **Purpose**: Measures the complete time for processing a single frame
- **Location**: Main game loop in `src/app/update.rs`
- **Significance**: Overall performance indicator - should stay under 16.67ms for 60 FPS

#### `frame_time`
- **Purpose**: Marker for individual frame timing analysis
- **Location**: End of frame processing
- **Significance**: Helps identify frame-to-frame performance variations

### 2. Rendering Benchmarks

#### `canvas_update`
- **Purpose**: Measures the time to update the main rendering canvas
- **Location**: `src/app/update.rs` - main rendering pipeline
- **Significance**: GPU command preparation and surface updates

#### `game_objects_rendering`
- **Purpose**: Measures the time to render all game objects (maze, enemies, UI)
- **Location**: `src/renderer/wgpu_lib.rs` - game rendering pass
- **Significance**: Core rendering performance indicator

#### `text_preparation`
- **Purpose**: Measures text rendering preparation time
- **Location**: `src/app/update.rs` - text renderer setup
- **Significance**: UI text performance

#### `text_rendering`
- **Purpose**: Measures actual text rendering time
- **Location**: `src/app/update.rs` - text render pass
- **Significance**: UI rendering performance

#### `command_encoder_creation`
- **Purpose**: Measures GPU command encoder creation time
- **Location**: `src/app/update.rs` - command preparation
- **Significance**: GPU command overhead

#### `command_submission`
- **Purpose**: Measures GPU command submission time
- **Location**: `src/app/update.rs` - command submission
- **Significance**: GPU synchronization overhead

#### `surface_presentation`
- **Purpose**: Measures surface presentation time
- **Location**: `src/app/update.rs` - frame presentation
- **Significance**: Display synchronization

#### `device_polling`
- **Purpose**: Measures GPU device polling time
- **Location**: `src/app/update.rs` - device maintenance
- **Significance**: GPU resource management overhead

### 3. Game Logic Benchmarks

#### `game_state_update`
- **Purpose**: Measures game state update time (player, enemy, audio)
- **Location**: `src/app/update.rs` - game state processing
- **Significance**: Core game logic performance

#### `audio_update`
- **Purpose**: Measures audio system update time
- **Location**: `src/app/update.rs` - audio manager updates
- **Significance**: Audio processing overhead

#### `enemy_pathfinding`
- **Purpose**: Measures enemy AI pathfinding calculations
- **Location**: `src/app/update.rs` - enemy update loop
- **Significance**: AI performance indicator

### 4. Collision Detection Benchmarks

#### `collision_detection_and_resolution`
- **Purpose**: Measures player collision detection and wall sliding
- **Location**: `src/game/collision.rs` - main collision method
- **Significance**: Physics performance - critical for smooth movement

#### `bvh_query_collisions`
- **Purpose**: Measures BVH tree traversal for collision queries
- **Location**: `src/game/collision.rs` - BVH query method
- **Significance**: Spatial partitioning performance

#### `cylinder_intersects_geometry`
- **Purpose**: Measures cylinder-geometry intersection tests
- **Location**: `src/game/collision.rs` - cylinder collision method
- **Significance**: Enemy pathfinding collision detection

#### `bvh_build`
- **Purpose**: Measures BVH tree construction time
- **Location**: `src/game/collision.rs` - BVH build method
- **Significance**: Level loading performance

#### `collision_system_build`
- **Purpose**: Measures complete collision system initialization
- **Location**: `src/game/collision.rs` - system build method
- **Significance**: Level loading performance

### 5. Maze Generation Benchmarks

#### `maze_generation`
- **Purpose**: Measures complete maze generation process
- **Location**: `src/app/update.rs` - maze generation loop
- **Significance**: Level creation performance

#### `maze_generation_steps`
- **Purpose**: Measures individual maze generation steps
- **Location**: `src/app/update.rs` - generation step loop
- **Significance**: Generation algorithm performance

#### `maze_completion_processing`
- **Purpose**: Measures maze completion and file saving
- **Location**: `src/app/update.rs` - completion handling
- **Significance**: Level finalization performance

#### `maze_geometry_generation`
- **Purpose**: Measures 3D geometry creation from maze data
- **Location**: `src/app/update.rs` - geometry generation
- **Significance**: Rendering setup performance

#### `enemy_placement`
- **Purpose**: Measures strategic enemy positioning
- **Location**: `src/app/update.rs` - enemy placement logic
- **Significance**: Level setup performance

### 6. AI Benchmarks

#### `enemy_pathfinding_update`
- **Purpose**: Measures enemy pathfinding algorithm execution
- **Location**: `src/game/enemy.rs` - pathfinding update method
- **Significance**: AI performance and complexity

## Performance Insights

The benchmark system provides several types of performance insights:

### 1. Most Time-Consuming Operations
- Identifies which operations consume the most total time
- Shows percentage of total measured time for each operation
- Helps prioritize optimization efforts

### 2. High Average Time Operations
- Identifies operations with average times > 1ms
- Indicates potential performance bottlenecks
- Helps identify operations that need optimization

### 3. Frequently Called Operations
- Identifies operations called > 1000 times
- Shows which operations are most critical for overall performance
- Helps identify hot paths in the code

## Usage

### Automatic Benchmarking
Benchmarks are automatically collected during gameplay and saved periodically (every 5000 frames).

### Manual Benchmark Saving
Press `F5` during gameplay to manually save benchmark results.

### Benchmark Results Location
Results are saved to `debug-analytics/benchmarks/` with timestamped filenames.

### Console Output
Performance summaries are printed every 1000 frames showing current FPS and basic metrics.

## Performance Targets

### Frame Time Targets
- **60 FPS**: < 16.67ms per frame
- **30 FPS**: < 33.33ms per frame
- **Minimum**: < 50ms per frame for playable experience

### Critical Operation Targets
- **Collision Detection**: < 1ms per frame
- **Rendering**: < 10ms per frame
- **AI Updates**: < 2ms per frame
- **Maze Generation**: < 100ms total (one-time cost)

## Optimization Guidelines

### High Priority (Critical Path)
1. **Collision Detection**: Directly affects player experience
2. **Rendering**: Affects visual performance and frame rate
3. **Frame Time**: Overall performance indicator

### Medium Priority (Frequent Operations)
1. **Enemy Pathfinding**: Affects AI responsiveness
2. **Game State Updates**: Core game logic performance
3. **Audio Updates**: Affects audio responsiveness

### Low Priority (One-time Operations)
1. **Maze Generation**: Only affects level loading
2. **BVH Building**: Only affects level loading
3. **Geometry Generation**: Only affects level loading

## Technical Implementation

### Benchmark Macros
- `crate::benchmark!(name, { code })`: Times a code block
- `crate::debug_benchmark!(name, { code })`: Only active in debug builds

### Profiler Integration
- `state.profiler.start_section(name)`: Start timing a section
- `state.profiler.end_section(name)`: End timing a section

### Data Collection
- Automatic collection during gameplay
- Periodic saving to prevent data loss
- Comprehensive metrics (count, total, average, min, max)

## Example Benchmark Output

```
=== PERFORMANCE SUMMARY ===
This report shows the most taxing parts of the Mirador game engine:

Operation                              | Count:   1234 | Total:     1.234s | Avg:    1.00ms | Min:    0.50ms | Max:    2.50ms
collision_detection_and_resolution     | Count:   1234 | Total:     0.500s | Avg:    0.40ms | Min:    0.20ms | Max:    1.00ms
game_objects_rendering                 | Count:   1234 | Total:     0.300s | Avg:    0.24ms | Min:    0.15ms | Max:    0.50ms
enemy_pathfinding                      | Count:   1234 | Total:     0.200s | Avg:    0.16ms | Min:    0.10ms | Max:    0.30ms

=== PERFORMANCE INSIGHTS ===
🏆 Most time-consuming operation: collision_detection_and_resolution (40.5% of total measured time)

⚠️  Operations with high average times (>1ms):
   • collision_detection_and_resolution: 1.00ms average

🔄 Most frequently called operations (>1000 calls):
   • collision_detection_and_resolution: 1234 calls
   • game_objects_rendering: 1234 calls
   • enemy_pathfinding: 1234 calls
```

This comprehensive benchmarking system provides detailed insights into the performance characteristics of the Mirador game engine, enabling targeted optimization efforts and performance monitoring. 