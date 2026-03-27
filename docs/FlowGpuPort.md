# Flow Field GPU Compute Port — Implementation Plan

Branch: `feature/flow-gpu-port`
Delete this file when the feature merges to dev.

## Context

The Flow theme's particle effect currently runs entirely on CPU:
- 3D curl noise (18 noise3d calls per particle per octave × 2 octaves)
- Particle advection (position integration, Z boundary, piece repulsion, shockwaves)
- Invisible tetrahedron capture system (5 tetras, collision detection, phase state machine)
- ~1000+ particles at runtime

The CPU version is in `src/gui/effects/flow_field.rs` (~800 lines). It stays as the
debuggable reference — refinements happen on CPU first, then get ported to GPU.

## Architecture Already in Place

- `GpuEffect` trait in `src/gui/effects/mod.rs` — `create_gpu_resources`, `compute`, `render_gpu`
- `EffectManager::dispatch_compute()` in `src/gui/effect_manager.rs` — called each frame
- `GpuState` exposes `device()`, `queue()`, `submit()` for GPU resource access
- `PostProcessChain` for fullscreen shader passes
- `MandelbrotUniforms` pattern — CPU pushes uniforms, GPU shader uses them

## What Moves to GPU

### Compute Shader (runs before render passes)
1. **Curl noise velocity field** — compute on a 3D grid (e.g., 32×32×8), particles sample it
2. **Particle advection** — update position/velocity from the velocity grid
3. **Tetra collision** — test each particle against 5 tetrahedra (20 face planes total)
4. **Stuck particle positioning** — move stuck particles with their tetra

### What Stays on CPU
- Phase state machine (Scatter → Attract → Assembled → Release → Pause)
- Tetra position/rotation updates (5 tetras × few floats, negligible)
- Spawn logic (decides how many particles to create)
- Audio analysis integration (already on CPU)
- Piece cell positions for repulsion
- Shockwave disturbance list

## GPU Resources Needed

### Storage Buffers
| Buffer | Format | Size | Description |
|--------|--------|------|-------------|
| particles | struct array | MAX_PARTICLES × ParticleGpu | position, velocity, life, hue, size, stuck_to, stuck_offset |
| velocity_grid | vec4 array | 32×32×8 | precomputed curl noise velocity at each grid point |

### Uniform Buffer
```rust
struct FlowUniforms {
    time: f32,
    dt: f32,
    noise_scale: f32,
    speed: f32,
    particle_count: u32,
    // Tetra data (5 tetras × 4 vertices × 3 floats = 60 floats)
    tetra_verts: [[f32; 3]; 20],  // 5 tetras × 4 verts
    tetra_count: u32,
    // Piece cells for repulsion
    piece_cells: [[f32; 2]; 4],
    piece_cell_count: u32,
    // Phase info
    capture_phase: u32,
    // Assembled rotation for stuck particle positioning
    assembled_center: [f32; 3],
    assembled_rotation: [f32; 3],
}
```

### Pipelines
1. **Velocity grid compute** — dispatches 32×32×8 workgroups, each computes curl noise at its grid point
2. **Particle advection compute** — dispatches ceil(particle_count/64) workgroups, each thread updates one particle
3. **Particle render pipeline** — vertex shader reads from particle storage buffer, emits billboard quads

## Implementation Steps

### Step 1: Particle Storage Buffer
- Define `ParticleGpu` struct (position, velocity, life, hue, size, stuck_to, stuck_offset)
- Create storage buffer on FlowField with `create_gpu_resources`
- CPU spawns particles by writing to a staging region of the buffer

### Step 2: Velocity Grid Compute
- WGSL compute shader: each thread computes `curl_noise_3d` at its grid coordinate
- Output to a 3D storage buffer (or flattened array)
- Dispatched once per frame before particle advection

### Step 3: Particle Advection Compute
- WGSL compute shader: each thread reads one particle, samples velocity grid (trilinear), integrates position
- Handles: Z boundary, piece repulsion (from uniform), stuck particle positioning
- Tetra collision: test against 20 face planes from uniform data

### Step 4: Particle Render Pipeline
- Vertex shader: reads particle data from storage buffer, emits billboard quad vertices
- Fragment shader: soft circle UV falloff (same as current)
- Replaces the CPU `render()` method's geometry generation

### Step 5: CPU↔GPU Sync
- CPU manages phase state, tetra positions, spawn count
- Each frame: CPU writes uniforms + spawns new particles → GPU computes → GPU renders
- `use_gpu()` flag for runtime toggle between CPU and GPU paths

## File Changes

| File | Changes |
|------|---------|
| src/gui/effects/flow_field.rs | Add GpuEffect impl, keep AudioEffect as fallback |
| src/gui/effect_manager.rs | Wire GPU dispatch for flow field |
| src/gui/renderer.rs | May need particle render pipeline (or use existing transparent) |

## Key Considerations

- **f32 noise precision**: GPU curl noise will use f32 (same as CPU version now)
- **Particle cap**: GPU can handle 10K+ particles easily — increase density significantly
- **Spawn pattern**: CPU decides spawn count, writes new particles to end of buffer. GPU needs an atomic counter or CPU tracks count via uniform.
- **Tetra collision in shader**: 5 tetras × 4 face planes = 20 plane tests per particle. Each is a dot product — very fast on GPU.
- **Stuck particle offset**: stored in particle buffer. During assembled phase, the compute shader applies assembled_rotation to the offset.

## Current CPU Flow Field Behavior Summary

### Particles
- Spawn at random positions in world bounds (-12..22 x, -25..5 y, -4..-0.5 z)
- Life: 6-14 seconds, hue from centroid ± 0.4
- Movement: 2-octave 3D curl noise, damped Z motion
- Piece repulsion, shockwave disturbances
- Stuck particles: lifetime paused, follow tetra rotation

### Tetrahedron Capture Cycle (~100 second total)
1. **Scatter (32s)**: 5 tetras tumble independently, capturing particles inside their volume
2. **Attract (6s)**: tetras converge to face-to-face assembly positions, spin decays, outer particles fade in last 0.5s
3. **Assembled (50s)**: all tetras rotate as one composite shape, capture continues (with 0.75s initial cooldown). All particles freed at transition with zero velocity.
4. **Release (2s)**: all stuck particles freed, zero velocity ghost
5. **Pause (10s)**: nothing, then restart

### Tetra Geometry
- 5 regular tetrahedra (TETRA_VERTS at ±1), scale 2.8
- Center (#4) + 4 outer ones that press face-to-face
- Outer vertices computed via `assembled_tetra_verts()` — reflects center's opposite vertex through each face plane
- Collision: `contains_explicit()` — same-side-as-opposite-vertex test for all 4 faces
- Spawn compensation: +10% spawn rate per stuck particle
