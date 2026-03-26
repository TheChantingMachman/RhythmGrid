# Scene/GUI Rearchitect — Feature Plan

Branch: `feature/scene-rearchitect`
Delete this file when the feature merges to dev.

## Goal

Decompose the GUI layer into focused, composable modules to support the roadmap:
GPU compute effects, background environments, post-processing chain, material system.

## Guiding Principles

- Each phase must leave the game fully playable — no "everything is broken until phase 4"
- Smoke test after every step: launch, play a game, cycle all themes, verify effects
- Preserve all existing visual behavior — this is refactoring, not redesign
- Pipeline-owned code (src/*.rs, tests/) is untouched. All changes in src/gui/

---

## Phase 1: Break Up the God Object

`GameWorld` has 107 fields and `tick()` is 440 lines. Decompose into focused structs.

### Step 1.1: Extract AudioAnalysis

Create `src/gui/audio_analysis.rs`.

Move from GameWorld:
- Fields: rolling_energy, beat_confidence, norm_ceil, bands_norm, peak_bands,
  band_beat_intensity, centroid, flux, energy_averages, confidence_values,
  resolved_ranks, ranks_locked, track_time, last_track_name, fft_buffer-related state
- Logic: all signal normalization, peak hold, beat intensity decay, rank resolution,
  track change detection from tick()

Interface:
```rust
pub struct AudioAnalysis { ... }
impl AudioAnalysis {
    pub fn update(&mut self, audio: &AudioState, dt: f64);
    pub fn audio_frame(&self) -> AudioFrame;
    pub fn resolve_rank(&self, rank: SignalRank) -> usize;
}
```

GameWorld holds `analysis: AudioAnalysis` and calls `self.analysis.update()` in tick().

**Smoke test**: Launch, play with music. Verify:
- Beat rings pulse on beat
- FFT visualizer responds to frequency bands
- Theme-specific effect bindings still work (rank resolution)
- Track change resets analysis state
- Debug dashboard shows correct energy/confidence values

### Step 1.2: Extract EffectManager

Create `src/gui/effect_manager.rs`.

Move from GameWorld:
- Fields: beat_rings, hex_background, fft_vis, grid_lines, fireworks, fire,
  starfield, aurora, flow_field, effect_flags, bindings, particles
- Logic: the if-chain effect dispatch from tick() and the matching dispatch in scene.rs

Interface:
```rust
pub struct EffectManager { ... }
impl EffectManager {
    pub fn new(theme: &VisualTheme) -> Self;
    pub fn update_all(&mut self, audio: &AudioFrame, flags: &EffectFlags, dt: f64, ...);
    pub fn apply_theme(&mut self, theme: &VisualTheme);
    pub fn render_background(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext);
    pub fn render_foreground(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext);
}
```

scene.rs calls `effects.render_background()` and `effects.render_foreground()` instead
of the manual if-chains. Adding a new effect becomes: add to EffectManager, no scene.rs change.

**Smoke test**: Launch, cycle all 5 themes. Verify:
- All effects render correctly per theme (fire on default, aurora on space, flow on flow, etc.)
- Beat-reactive effects still pulse
- Flow field piece repulsion and shockwaves still work
- Particles spawn on line clears and beats
- Theme switching clears/resets effects correctly

### Step 1.3: Extract Animations

Create `src/gui/animations.rs`.

Move from GameWorld:
- Fields: clearing_cells, drop_trails, settle_cells, shatter_fragments, bg_rings,
  t_spin_flash, level_up_flash
- Logic: animation spawning (from hard_drop handler, line clear handler) and
  decay/physics updates from tick()
- Associated constants: LINE_CLEAR_DURATION, DROP_TRAIL_DURATION, SETTLE_DURATION, etc.
- Associated structs: ClearingCell, DropTrail, SettleCell, ShatterFragment, BgRing

Interface:
```rust
pub struct Animations { ... }
impl Animations {
    pub fn update(&mut self, dt: f32);
    pub fn spawn_line_clear(&mut self, ...);
    pub fn spawn_hard_drop(&mut self, ...);
    pub fn spawn_settle(&mut self, ...);
    // Accessors for scene.rs to read animation state
}
```

**Smoke test**: Launch, play a full game. Verify:
- Hard drop trails render and fade
- Line clear dissolve animation works
- Shatter fragments fly out on clears
- Settle squish on landing
- T-spin flash
- Level-up ring burst
- Camera shake on drops/clears

### Phase 1 Checkpoint

After all three extractions, verify:
- `GameWorld` struct has ~40-50 fields (down from 107)
- `tick()` is ~100-150 lines (down from 440)
- All themes, effects, and animations work identically to before
- No new warnings
- All pipeline tests still pass (`cargo test`)

---

## Phase 2: Shader and Renderer Cleanup

### Step 2.1: Deduplicate WGSL Lighting

Extract the shared lighting model (~60 lines: GGX specular, environment reflection,
rim light) into a `const LIGHTING_WGSL: &str`. Concatenate it into both the scene
shader and OIT shader at build time:
```rust
let scene_src = format!("{}\n{}", LIGHTING_WGSL, SCENE_MAIN_WGSL);
```

**Smoke test**: Launch. Verify:
- Board pieces have same specular/reflection/rim as before
- OIT transparent pieces match (same lighting)
- Bloom still picks up HDR values

### Step 2.2: Persistent GPU Buffers

Replace per-frame `create_buffer_init` with reusable buffers that grow as needed.
Keep a `max_verts` / `max_indices` high-water mark, only reallocate when exceeded.
Use `queue.write_buffer` for data upload.

**Smoke test**: Launch, play for 2+ minutes. Verify:
- No visual differences
- No GPU errors / validation layer complaints
- Memory usage stable (not growing per frame)

### Step 2.3: PostProcessChain

Extract bloom from `render()` into a `PostProcessPass` struct. Create `PostProcessChain`
with ping-pong texture pair. The render loop calls `chain.execute()`.

Initially the chain has one entry (bloom+tonemap+color grade — the existing pass).
Future passes (vignette, chromatic aberration, distortion) are added as chain entries.

**Smoke test**: Launch, verify:
- Bloom looks identical
- Color grading per theme still works
- No artifacts from ping-pong texture management

### Phase 2 Checkpoint

- Lighting changes are one-edit in one place
- Per-frame buffer allocation eliminated
- Post-process chain is extensible
- All visuals identical to pre-refactor

---

## Phase 3: GPU-Aware Effect Interface

### Step 3.1: GpuEffect Trait

Add alongside AudioEffect (not replacing it):
```rust
pub trait GpuEffect {
    fn create_resources(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    fn compute(&mut self, encoder: &mut wgpu::CommandEncoder, audio: &AudioFrame);
    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}
```

EffectManager checks: does this effect implement GpuEffect? If so, dispatch compute +
render. If not, fall through to CPU AudioEffect path.

**Smoke test**: Launch — no behavioral change. The trait exists but nothing uses it yet.

### Step 3.2: Expose GPU Access

EffectManager receives `&wgpu::Device` and `&wgpu::Queue` references (or an `Arc`).
GpuEffects can create persistent buffers, bind groups, compute pipelines.

Update render() to insert a compute pass before the OIT accumulation pass.
GpuEffects dispatch their compute work there.

**Smoke test**: Launch — no behavioral change. The compute pass exists but is empty.

### Step 3.3: Port Flow Field to GPU Compute

First concrete GpuEffect. The flow field moves from CPU particle sim to:
1. Compute shader: curl noise velocity grid + particle advection
2. Render: indirect draw from GPU particle buffer

The CPU flow field code becomes the reference implementation / fallback.

**Smoke test**: Launch, switch to Flow theme. Verify:
- Particles still swirl with curl noise
- Audio reactivity preserved (spawn rate, turbulence, centroid color)
- Piece repulsion still works
- Hard drop shockwaves still work
- CPU usage drops significantly on Flow theme
- Particle count can be increased dramatically

### Phase 3 Checkpoint

- One effect running on GPU compute
- CPU and GPU effect paths coexist
- Framework ready for fluid dynamics, more GPU particles

---

## Phase 4: Scene Structure

### Step 4.1: Extract Dashboard UI

Create `src/gui/dashboard.rs`. Move the 200+ lines of button layout, volume bar,
transport controls, folder icon, track queue from scene.rs.

**Smoke test**: Launch, interact with all dashboard controls (volume, skip, shuffle,
folder picker, theme cycle). Verify all buttons respond correctly.

### Step 4.2: BackgroundLayer Trait

Create a trait or enum for themed background environments. Each background:
- Owns its geometry and potentially its own shader/pipeline
- Renders in the opaque pass before the board (depth write ON)
- Is selected per-theme in VisualTheme

Migrate existing backgrounds (hex grid, starfield) to this interface.
Future backgrounds (underwater, cityscape) implement the same trait.

**Smoke test**: Launch, cycle themes. Verify:
- Hex background on water theme
- Starfield on space theme
- Board correctly occludes backgrounds

### Step 4.3: Material Parameters

Add a material uniform buffer (or extend Uniforms) with:
roughness, metallic, specular_intensity, emissive_strength.

Themes write material presets. Shader reads from uniform instead of hardcoding.

**Smoke test**: Launch, cycle themes. Verify:
- Default theme has current look (roughness 0.3)
- Can tweak theme material params and see difference
- OIT shader reads same material params

---

## File Impact Summary

| New File | Purpose |
|----------|---------|
| src/gui/audio_analysis.rs | Audio signal processing pipeline |
| src/gui/effect_manager.rs | Effect lifecycle, dispatch, theme switching |
| src/gui/animations.rs | Game animation state and physics |
| src/gui/dashboard.rs | Dashboard UI geometry building |

| Modified File | Scope |
|---------------|-------|
| src/gui/world.rs | Major — decompose into delegating to new modules |
| src/gui/scene.rs | Major — delegate to effect_manager and dashboard |
| src/gui/renderer.rs | Medium — persistent buffers, post-process chain, compute pass |
| src/gui/effects/mod.rs | Minor — add GpuEffect trait |
| src/gui/mod.rs | Minor — add new module declarations |

| Unchanged | |
|-----------|---|
| src/gui/drawing.rs | Vertex/geometry primitives stay as-is |
| src/gui/app.rs | Event loop stays as-is |
| src/gui/camera.rs | Camera reactor stays as-is |
| src/gui/effects/*.rs | Individual effects stay as-is (until GPU port) |
| src/*.rs, tests/*.rs | Pipeline-owned, untouched |
