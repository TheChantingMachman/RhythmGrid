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
- Streaming decode for near-instant track transitions
- 7-band FFT decomposition (sub-bass through brilliance)
- Multi-band beat detection (7 independent detectors, one per band)
- Per-band normalization (equal visual weight regardless of absolute energy)
- Peak hold tracking per band
- Spectral centroid and flux analysis (pipeline, pending delivery)
- Energy/amplitude tracking per frame

### Tetris Core
- Standard Tetris mechanics: tetrominoes, rotation (SRS), line clearing, gravity
- Increasing speed / level progression
- Hold piece (C/Shift) with 3D rotating preview
- Lock delay (400ms, 15 resets max)
- T-spin detection with bonus scoring + visual flash
- Combo system (consecutive line clear multiplier + counter display)
- Per-game statistics (pieces placed, max combo, time played)
- **Audio does NOT affect difficulty** — music is purely visual/atmospheric, gameplay speed is independent
- **2-stage panic escalation** (NES Tetris style):
  - Normal — standard playback, baseline particle effects and beat pulse
  - Danger — music playback speeds up, particle effects intensify

### Music Integration
- **No music configured:** Procedurally generated audio for out-of-box experience and first-time players. Zero licensing issues at any release tier, deterministic output for pipeline testing.
- **Basic (V1):** Point at a folder via path field or filesystem browser. Play files sequentially or shuffled. Seamless auto-advance between tracks.

**Future:** Playlist support, queue management, metadata-driven theming

### Visual Layer
- **3D rendering** — volumetric blocks, Blinn-Phong lighting, bloom post-processing, proper depth testing with opaque + transparent passes
- **Block appearance** — V1: plain cubes with per-piece-type frequency band glow. Future: pixel art tiles, swappable block visuals.
- **Board pulse** — cube depth modulated by per-band beat intensity, board is a full-spectrum visualizer
- **Camera:** Fixed perspective with beat-driven bass sway, hi-freq jitter, and impact shake
- **Particle effects** — small dense particles. Line clear spray (120 particles), beat burst from board edges, level-up radial burst. Per-band beat triggers: bass → rings, upper-mids → particles.
- **Background** — rotating hex dot grid (breathes with low-mids, warms with sub-bass), connecting lines, expanding beat rings
- **Grid** — transparent wireframe overlaid on cubes, shimmer driven by presence band + beats
- **FFT visualizer** — 7-band spectral display with peak hold indicators, spectral color gradient, lockable visibility
- **HUD** — auto-fading, core game controls don't wake HUD, secondary controls do
- **Escalation:** all effects intensify with danger level (ring speed, sway amplitude, hex rotation)
- Future: color palette shifts, visual themes as effect module bundles, spectral centroid-driven color temperature

### Dynamic Audio-Visual Mapping (phased)
Real-time song fingerprinting to make visuals respond to what's musically interesting, not just loud.

**Done:**
- Peak hold indicators on all 7 FFT bands
- Per-piece-type band glow (each tetromino pulses with a different band)
- Background elements routed to individual bands (grid=presence, dots=low-mids/sub-bass)
- 7-band FFT visualizer with spectral color gradient

**Known issues:**
- Most songs concentrate energy in the lower 5 bands (sub-bass through upper-mids). Presence and brilliance often flat. May need per-band normalization or logarithmic scaling.
- Beat detection is RMS-only — misses rhythmic content in specific bands

**Beat detection overhaul:**
- Current: single RMS amplitude spike detection. Cheap but misses a lot.
- Goal: budget multi-band beat detection that's still computationally cheap
- Run spike detection per-band (sub-bass for kicks, upper-mids for snares, presence for hi-hats)
- Beat event typed by source band — enables band-specific visual responses (kick → ring, snare → flash, hat → shimmer)
- Requires `audio.beat_detect` spec update + pipeline rebuild
- Open: threshold tuning per band — bass needs different sensitivity than highs
- Open: should beat events carry intensity, or just binary on/off?
- This has huge impact on look and feel — priority design decision

**Visual effects interface:**
- Inventory all GUI elements currently hooked to audio data (TODO)
- Define a trait/interface for "visual effect module" — takes audio state, outputs render commands
- Effects can be swapped, layered, or assigned to bands dynamically
- 7-band FFT visualizer doubles as a debug view for the interface — shows exactly what data each effect is receiving
- Themes (from Architecture Note) become bundles of effect modules with preset band assignments

**Rolling averages + dominant band ranking:**
- Exponential moving average per band (7 floats, ~5-10s settling period)
- Sort to find top 3-5 most active bands per song section
- Open: how to handle ranking transitions mid-song (verse→chorus)? Hard cutover vs smooth crossfade?
- Open: should settling period reset on track change?

**Effect routing by dominant band:**
- Map top-N active bands to effect modules dynamically
- Board color tinting shifts based on current energy profile
- Escalation effects modulated by which bands are hot, not just stack height
- Open: how many elements to assign? More = richer but harder to notice

**Per-song adaptation:**
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

**Done:**
- Idle/visualizer/demo mode (auto-play after 15s, auto-restart on game over)
- HUD fade refinements (1.5s timer, core controls don't wake)
- 7-band FFT with spectral color gradient + peak hold
- Multi-band beat detection (7 independent detectors)
- Per-band visual routing (bass→rings, upper-mids→particles, presence→grid shimmer)
- Per-piece-type band glow + board pulse (cubes are full-spectrum visualizer)
- Per-band normalization for equal visual weight
- Hold piece with 3D preview
- T-spin detection + combo system with visual feedback
- Game over screen with stats (score, combo, pieces, time)
- Streaming decode for instant track transitions
- Proper depth testing (opaque + transparent passes)
- Camera bass sway + hi-freq jitter + bass zoom
- Guideline key bindings (X=RotateCW, C=Hold, GameAction::Hold)
- Spectral centroid + flux signals (pipeline delivered)
- Board pulse (cube depth modulated by per-band beat)
- Grid line thickness pulse on presence beats
- Dashboard elements in transparent pass (no black boxes on fade)
- Bass zoom (camera Z push on bass beats)

**Remaining:**
- Effects interface: AudioEffect trait defined, 4 audio effects + CameraReactor migrated. Theme struct ready but not wired. See AudioVisualCatalog.md for migration status.
- Build a second theme to prove the system (different ring style, hex pattern, camera behavior) — Water theme params defined, needs piece color overrides
- Theme piece color overrides: `themed_piece_color()` wrapper that checks theme override before falling back to pipeline `piece_color()`. Touches ~8 render call sites.
- Spectral centroid wired to grid color temp (done). Spectral flux wired to background brightness (done). More routing possible.
- Rolling averages + dominant band ranking (infrastructure for dynamic effect routing)
- Settings persistence (volume, shuffle state survive restart)
- 3D elements replacing 2D HUD overlays:
  - Shaped transport buttons (play triangle, pause bars, skip arrows)
  - Button press animation (depth halve on click)
  - Game over / pause as 3D elements
  - Score/level/lines as 3D floating text
- Responsive layout: side assemblies track window edges
- XDG-compliant config and data paths
- Linux packaging (Flatpak, Snap, AUR)
- Future: user-remappable key bindings, 3D in-game filesystem browser, multi-band beat tuning per-band thresholds

---

*This plan will be refined as open questions are resolved and architecture takes shape.*
