# RhythmGrid — Completed Work & Historical Reference

This document captures completed features, resolved design decisions, and implemented techniques. Moved from ProductPlan.md to keep the active plan focused.

## Resolved Design Decisions

- **Language/platform** — Rust
- **Does music affect difficulty?** — No, audio is purely visual/atmospheric
- **Song transition behavior** — Seamless auto-advance
- **Layout** — Board-centered: game HUD left, music dashboard right (3D elements)
- **Panic escalation** — 2-stage (normal / danger), NES style
- **Scope of audio analysis** — Started lightweight (amplitude/BPM), expanded to 7-band FFT + spectral centroid + flux
- **Bundled fallback** — Embedded default track. Zero licensing risk, works out of box.
- **Single-player only?** — Yes. Multiplayer is distant-future roadmap at best.
- **Idle/demo mode AI** — Random placement for V1. Future: music-reactive bot.

## Completed Phases

### Phase 1 — Foundation (complete)
- Cargo project setup and module structure
- Audio file loading, decoding, and playback
- Real-time amplitude and beat detection
- wgpu 3D rendering (board-centered layout, Blinn-Phong lighting, bloom)
- Playable Tetris with audio reactivity (beat pulse, FFT visuals, particles)
- 3D music dashboard: volume bar, FFT visualizer, track name, control hints

### Phase 2 — The Effect (complete)
- Wire audio analysis into visual layer
- Particle effects and beat pulse
- 2-stage panic escalation (speed up music, intensify effects)
- Bundled fallback track (debug + first-time experience)
- Music folder selection via native OS dialog (3D button + rfd crate)
- Beat-driven camera zoom on bass beats

### Phase 3 — Polish (largely complete)
- Idle/visualizer/demo mode (auto-play after 15s, auto-restart on game over)
- HUD fade refinements (1.5s timer, core controls don't wake)
- 7-band FFT with spectral color gradient + peak hold
- Multi-band beat detection (7 independent detectors)
- Per-band visual routing (bass->rings, upper-mids->particles, presence->grid shimmer)
- Per-piece-type band glow + board pulse (cubes are full-spectrum visualizer)
- Per-band normalization for equal visual weight
- Hold piece with 3D preview
- T-spin detection + combo system with visual feedback
- Game over screen with stats (score, combo, pieces, time)
- Streaming decode for instant track transitions
- Ghost piece (translucent landing projection)
- Camera bass zoom (no lateral sway — causes motion sickness)
- Guideline key bindings (X=RotateCW, C=Hold)
- Spectral centroid + flux signals
- Board pulse (cube depth modulated by per-band beat)
- Grid line thickness pulse on presence beats
- Dashboard elements in transparent pass
- Effects interface: AudioEffect trait, CameraReactor, EffectFlags (16 toggles)
- Theme piece color overrides: themed_piece_color() with per-theme palettes
- Theme switching: F1 cycles through 9 themes with toast notification
- Rolling averages + dominant band ranking
- Dynamic audio-visual mapping: EffectBindings + SignalRank system
- Two-phase analysis (7s initial lock, 30-45s resample)
- Settings persistence (volume, theme, shuffle, music folder, window size)
- Render state layer (BoardRenderState, GameStatusRender, HeldPieceRender)
- Theme auto-cycle (4 minutes, skips Debug, F1 resets timer)

## Scene Architecture (v0.2.0)
- Decomposed 107-field GameWorld into: AudioAnalysis, EffectManager, Animations, Dashboard
- tick() reduced from 440 to ~200 lines
- Persistent GPU buffers (GpuBufferPair) with grow-on-demand
- Deduplicated WGSL lighting into shared compute_lit_color() (GGX specular, env reflection, rim light)
- PostProcessChain: composable fullscreen pass sequence with ping-pong textures
- GpuEffect trait for compute shader effects coexisting with CPU AudioEffect

## Rendering Techniques (implemented)

| Technique | Notes |
|---|---|
| GGX specular (Cook-Torrance BRDF) | Replaced Blinn-Phong |
| Per-face vertex color gradient | Per-cube face coloring |
| Weighted Blended OIT | Order-independent transparency for all transparent geometry |
| Per-band glow modulation | Each piece type pulses with a different frequency band |
| Beat depth pulse | Cube Z modulated by beat intensity |
| Bloom post-processing | smoothstep(0.35, 0.7), strength 0.6 |
| MSAA 4x | Multi-sample anti-aliasing |
| HDR + Rgba16Float | Float color space throughout pipeline |
| Fake subsurface scattering | Back-face brightening scaled by darkness |
| Inner glow (geometry) | HDR emissive cube inside translucent shell, low alpha/high HDR for OIT |
| Full bevel (12 edges + corners) | All cube edges rounded with corner patches |
| Contact AO | Per-vertex darkening from neighbor bitmask |
| Environment reflection | Procedural gradient sampled by reflection vector |
| Motion trails | Hard drop streak + subtle fall ghost |
| GPU Mandelbrot | Fullscreen fractal zoom with 8 targets, 3 palettes |
| GPU Flow compute | 32x32x8 velocity grid + 262K particle advection via WGSL compute |

## Visual Themes (9 total, v0.2.0)

1. **Default** — hex grid, beat rings, fireworks, starfield
2. **Water** — blue palette, hex grid, beat rings
3. **Space** — dark, starfield-dominant, subtle grid
4. **Flow** — 3D curl noise particles, invisible tetrahedron capture system (GPU compute)
5. **Fluid** — momentum-based particles, tumbling tetromino piece turbulence
6. **Crystal** — recursive elongated diamond fractals, explosion cycle, black volumetric fog
7. **Fractal** — GPU Mandelbrot zoom, ragdoll stick figure (verlet physics)
8. **Pipes** — 7 neon tubes per frequency band, right-angle 3D grid growth
9. **Debug** — all effects + analysis dashboard

## Preview/Hold Piece Rendering (complete)
Moved from HUD to world-space 3D via push_cube_3d — full bevels, inner glow, GGX lighting, 3-axis rotation. Weighted Blended OIT fixed all transparent z-ordering. Inner glow tuned with low alpha (0.15) / high HDR (1.4x) to avoid OIT color dilution.

## Flow Field GPU Port (v0.2.0)
- 32x32x8 velocity grid computed per frame via WGSL compute shader
- Particle advection, tetra collision, and billboard rendering all GPU-side
- 262K particle ring buffer (16MB), CPU retains phase state machine + spawn decisions
- Remaining: piece repulsion and shockwave disturbances not yet ported to GPU
