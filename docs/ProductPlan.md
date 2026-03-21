# RhythmGrid — Product Plan

## Vision

RhythmGrid is a Tetris game with audio-reactive visuals, driven by the player's own music library. The core gameplay is classic Tetris — tetrominoes, rotation, line clearing, gravity — wrapped in a synesthetic experience where everything pulses, shifts, and evolves with the music. When nobody's playing, it doubles as a music player and visualizer — arcade idle-mode style.

## Core Concept

- **Classic Tetris mechanics** — the game everyone knows, executed well
- **Bring your own music** — players point at their local music library
- **Audio-reactive visuals** — beat detection, frequency analysis, and energy tracking drive the visual layer in real time
- **Every session is unique** — different music creates a different visual experience
- **Dual-purpose** — playable game or idle visualizer/music player

## Target Audience

- Music lovers who enjoy classic puzzle games
- Players looking for a meditative, audio-visual experience
- Linux gamers (primary), with broader platform reach as a goal

## Release Tiers

### Tier 1 — Personal use (primary)
Just a fun game to play. Move fast, no licensing constraints yet.

### Tier 2 — Linux distribution
Package for Flathub, Snap, AUR, and other Linux stores. Requires:
- Clean asset licensing (bundled audio, fonts, etc.)
- XDG-compliant paths (`~/.config/rhythmgrid`, `~/.local/share/rhythmgrid`)
- System-friendly dependencies (PulseAudio/PipeWire)
- Single-binary or simple package

### Tier 3 — Steam (nice to have, don't preclude)
- Keep audio/rendering abstractable (no hard Linux-only API deps)
- Steamworks SDK integration is Rust-possible, defer until relevant

### Tier 4 — Beyond
- Other storefronts, platforms, etc. Don't actively plan for, but don't make impossible.

**Design principle:** Avoid decisions that preclude Tiers 1–2. Keep Tier 3 feasible.

## Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| Language | Rust | Strong audio/graphics ecosystem, proven in Dark Factory pipeline |
| Rendering | wgpu | Cross-platform (Vulkan/Metal/DX12), no platform lock-in |
| Windowing | winit | Cross-platform, pairs with wgpu |
| Audio playback | cpal | Cross-platform output (PulseAudio/PipeWire/ALSA) |
| Audio decoding | symphonia | Pure Rust, supports MP3/WAV/FLAC/OGG |
| Engine | None | Pipeline-friendly — every module is a testable Rust crate |

## Layout

Board-centered, clean minimalistic 3D (Tetris Effect style):

### Center — Game Board
- **3D game board in 3D space** — pieces are volumetric blocks, not flat sprites
- Board sits front and center in a 3D scene with lighting and depth
- Particle effects exist in 3D space (fly around/behind the board)
- Audio-reactive particle effects and beat pulse
- In idle/visualizer mode: game plays itself (demo mode)

### Left Side — Game HUD
- Score, Level, Lines (2D overlay text, top-left)
- Next piece preview (3D rotating cube)

### Right Side — Music Dashboard
- Track name (2D overlay text)
- Volume bar (3D slab, responds to +/- keys)
- FFT visualizer (3D columns — bass/mids/highs, audio-reactive)
- 3D button cubes: play/pause, back, skip, shuffle, folder picker
- All elements fade with HUD auto-hide, audio controls reveal HUD

**Future:**
- 3D in-game filesystem browser (replaces native OS dialog)
- Playlist management, queue editing, reordering
- Track metadata display (artist, album, artwork)

## Key Features

### Audio Engine
- Load and decode local audio files (MP3, WAV, FLAC, OGG)
- Real-time beat detection and BPM estimation
- Frequency band decomposition (bass, mids, highs)
- Energy/amplitude tracking per frame

### Tetris Core
- Standard Tetris mechanics: tetrominoes, rotation (SRS), line clearing, gravity
- Increasing speed / level progression
- **Audio does NOT affect difficulty** — music is purely visual/atmospheric, gameplay speed is independent
- **2-stage panic escalation** (NES Tetris style):
  - Normal — standard playback, baseline particle effects and beat pulse
  - Danger — music playback speeds up, particle effects intensify

### Music Integration
- **No music configured:** Procedurally generated audio for out-of-box experience and first-time players. Zero licensing issues at any release tier, deterministic output for pipeline testing.
- **Basic (V1):** Point at a folder via path field or filesystem browser. Play files sequentially or shuffled. Seamless auto-advance between tracks.

**Known issues:**
- Track transitions are slow — full decode before playback starts. Need chunked streaming so playback begins within ~100ms of track change.
- Silence gap during transition feels like the app is broken. Need visual feedback (loading indicator or immediate track name update).
- Transport button clicks sometimes miss on first press — hover detection may lag one frame behind click event.

**Future:** Playlist support, queue management, metadata-driven theming

### Visual Layer
- **3D rendering** — volumetric blocks, basic lighting/shading, camera in 3D space
- **Block appearance** — V1: plain cubes. Future: small pixel art tiles (8x8, 16x16, or 32x32) that define block faces, projections, or extruded shapes. Blocks should not be architecturally locked to "plain cube" — the rendering should support swappable block visuals.
- **Camera:** Fixed perspective for V1. Future: reactive camera movement on beats/escalation, player-controlled orbit/zoom
- **Screen shake** — triggered on line clears and hard drops
- **Momentum/inertia** — board slightly overshoots and settles back on impacts (delayed spring effect)
- **Particle effects** — primary visual expression, 3D particles in the scene, scales with audio intensity
- **Beat pulse effect** — grid/elements pulse on detected beats
- **Escalation:** all effects intensify as stack height enters danger zone
- Future: color palette shifts, multiple visual themes

### Dynamic Audio-Visual Mapping (phased)
Real-time song fingerprinting to make visuals respond to what's musically interesting, not just loud.

**Phase A — Peak hold indicators (GUI-side, current 3 bands):**
- Track decaying peak per FFT band, render as thin slab at peak height on each column
- Gives visual persistence and shows where energy lives in a song

**Phase B — Expand to 5 bands + rolling energy ranking:**
- Update `audio.fft` spec: sub-bass, bass, mids, upper-mids, highs
- Rolling window (5-10s) cumulative energy per band → dominant band detection
- Detects verse/chorus/breakdown transitions in real time

**Phase C — Effect routing by dominant band:**
- Map dominant band to visual intensity: bass → beat rings, mids → grid shimmer, highs → particle sparkle
- Board color tinting shifts based on current energy profile
- Escalation effects modulated by which bands are hot, not just stack height

**Phase D — Per-song adaptation:**
- Track cumulative fingerprint across full song playback
- Auto-tune effect sensitivity to each song's dynamic range
- Normalize quiet vs loud tracks for consistent visual intensity

### Architecture Note — Effects Modularity
Sound effects and visual effects should be behind trait interfaces, not hardcoded. A "theme" is conceptually a bundle of: particle behavior, color palette, sound effect set, camera behavior, and block appearance (pixel art tiles). V1 ships one theme with plain cubes, but the architecture doesn't preclude adding theme packs later. No plugin system needed — just clean trait boundaries.

### Player Experience
- Simple onboarding — start playing immediately with the bundled track
- Music setup is optional and can happen anytime via the dashboard
- Idle/visualizer mode — game plays itself when no input, music keeps going
- Song transitions: seamless auto-advance to next track

## Open Questions

- [x] Language/platform — **Rust**
- [x] Does music affect difficulty? — **No, audio is purely visual/atmospheric**
- [x] Song transition behavior — **Seamless auto-advance**
- [x] Layout — **Board-centered: game HUD left, music dashboard right (3D elements)**
- [x] Panic escalation — **2-stage (normal / danger), NES style**
- [x] Scope of audio analysis — **Start lightweight (amplitude/BPM). Add FFT/spectral when visuals demand it.**
- [x] Bundled fallback — **Procedural generation. Zero licensing, deterministic, pipeline-testable.**
- [x] Single-player only? — **Yes. Multiplayer is distant-future roadmap at best, not part of the core vibe.**
- [x] Idle/demo mode AI — **Random placement for V1. Future: greedy bot + session replay.**

## Phases (High-Level)

### Phase 1 — Foundation
- Cargo project setup and module structure
- Audio file loading, decoding, and playback
- Real-time amplitude and beat detection
- wgpu 3D rendering (board-centered layout, Blinn-Phong lighting, bloom)
- Playable Tetris with audio reactivity (beat pulse, FFT visuals, particles)
- 3D music dashboard: volume bar, FFT visualizer, track name, control hints

### Phase 2 — The Effect
- Wire audio analysis into visual layer
- Particle effects and beat pulse
- 2-stage panic escalation (speed up music, intensify effects)
- Bundled fallback track (debug + first-time experience)
- Music folder selection via native OS dialog (3D button + rfd crate)
- Beat-driven camera sway and impact shake on line clears/hard drops
- Future: 3D in-game filesystem browser replaces native dialog

### Phase 3 — Polish & Ship
- Idle/visualizer/demo mode
- Visual themes and palette system
- Settings (audio sensitivity, visual intensity, controls)
- Align key bindings to Tetris Guideline (X=RotateCW, C/Shift=Hold) — requires `game.hold_piece` first, then update `input.key_map` spec
- Decrease HUD fade timer. Core game controls (move, rotate, drop) should NOT de-fade the HUD; non-core mapped keys (audio controls, hold, pause) should reveal HUD
- XDG-compliant config and data paths
- Linux packaging (Flatpak, Snap, AUR)
- Expand FFT from 3 bands to 7 (sub-bass, bass, low-mids, mids, upper-mids, presence, brilliance) — `audio.fft` spec updated, pipeline rebuild + GUI visualizer update pending
- Shaped transport buttons (play triangle, pause bars, skip arrows) replacing square placeholders — remove text labels once shapes are self-explanatory
- Button press animation: halve depth on click to simulate depression
- Responsive layout: side assemblies track window edges rather than fixed world-space positions
- Future: user-remappable key bindings (settings UI + persisted config)

---

*This plan will be refined as open questions are resolved and architecture takes shape.*
