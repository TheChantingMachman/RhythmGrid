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

**Next (medium-low priority):** Track queue display near audio controls — show upcoming tracks as a small clickable list. Clicking a track plays it next. Respects shuffle order. Graduates into full playlist management later.

**Future:** Full playlist support (create, save, reorder), queue management, metadata-driven theming

### Visual Layer
- **3D rendering** — volumetric blocks, Blinn-Phong lighting, bloom post-processing, proper depth testing with opaque + transparent passes
- **Block appearance** — V1: plain cubes with per-piece-type frequency band glow.
- **Bitmap-extruded blocks** (future):
  - 2D pixel art bitmap (8x8 or 16x16) defines the block silhouette
  - Each filled pixel extruded into a mini-voxel column — front looks like pixel art, side/top has depth
  - Same lighting/glow/transparency/bloom applies to each mini-voxel automatically
  - Library of normalized bitmaps: `[u8; 8]` per shape, trivial to add new ones
  - Per-theme block sets: Retro (ghost, invader, heart), Nature (leaf, snowflake), Abstract (spiral, ring)
  - Swappable per-theme — the VisualTheme holds a block set reference
  - Render cost: 8x8 = up to 64 mini-cubes per cell vs 1 cube currently. May need LOD for dense boards — simplify to full cube when many cells occupied.
- **Rounded-edge cubes:**
  - Bevel sharp edges with extra vertex strips. ~56-80 verts per cube vs 24 currently.
  - Smooth normals at bevels create rolling specular highlights — instant polish upgrade.
  - Existing Blinn-Phong shader picks it up automatically, no shader change needed.
  - Could be a theme option: sharp cubes (default), beveled cubes (polished), heavily rounded (pill-shaped).
- **Metallic cubes:**
  - High specular intensity, tight highlight (high Phong exponent), low diffuse.
  - Sharp bright spot slides along surface as camera moves — shiny metal look.
  - Best combined with rounded edges so the highlight rolls across bevels.
  - Needs per-vertex material attributes (specular intensity + exponent) or shader uniforms per-material.
  - Theme variants: matte (current), metallic, chrome, brushed steel.
- **Block material quality** (Tetris Effect style, phased):
  - Semi-transparent cubes with visible back faces (crystalline volume feel)
  - Edge glow / fresnel effect (edges brighter than centers)
  - Per-face color gradient (not uniform flat color)
  - Beat-driven material modulation (specular shift, bloom intensity, saturation independently)
  - Theme-driven material presets (glass, gem, neon, matte)
- **Board pulse** — cube depth modulated by per-band beat intensity, board is a full-spectrum visualizer
- **Camera:** Fixed perspective with beat-driven bass sway, hi-freq jitter, and impact shake
- **Particle effects** — small dense particles. Line clear spray (120 particles), beat burst from board edges, level-up radial burst. Per-band beat triggers: bass → rings, upper-mids → particles.
- **Background** — rotating hex dot grid (breathes with low-mids, warms with sub-bass), connecting lines, expanding beat rings
- **Grid** — transparent wireframe overlaid on cubes, shimmer driven by presence band + beats
- **FFT visualizer** — 7-band spectral display with peak hold indicators, spectral color gradient, lockable visibility
- **HUD** — auto-fading, core game controls don't wake HUD, secondary controls do
- **Escalation:** all effects intensify with danger level (ring speed, sway amplitude, hex rotation)
- Future: color palette shifts, visual themes as effect module bundles, spectral centroid-driven color temperature

### Advanced Visual Techniques (to explore)
Inspired by Tetris Effect and Geometry Wars — techniques to consider as the visual layer matures:

**Grid distortion (Geometry Wars style):**
- Displace grid line vertices by force fields (beat sources, line clear shockwaves, stack weight)
- Board wireframe physically warps with the music — highest impact single effect
- Fits naturally into GridLines effect module (vertex displacement during render)

**Particle trails / ribbons (Geometry Wars style):**
- Render ribbon of quads connecting particle's last N positions instead of single quad
- Neon light-painting look — transforms fireworks and beat particles
- Moderate effort — track position history per particle, generate ribbon geometry

**Additive blending pass:**
- Separate render pass with additive blend state for rings, particles, fireworks
- Overlapping bright elements get brighter, not more opaque — electric neon feel
- Small effort — new pipeline variant, render select effects into it

**Screen-space distortion:**
- Post-process pass that warps UVs on impacts (barrel distortion on line clears)
- Adds physical punch — screen itself reacts to game events
- Larger effort — new post-process pass after bloom

**Subsurface scattering approximation:**
- Light appears to pass through semi-transparent cubes
- Combined with back-face tinting gives gemstone/crystal quality
- Shader technique — approximate with view-dependent color shift

**Water surface background (theme: Ocean/Calm):**
- Dense grid of vertices with sine-wave Y displacement — rolling waves
- Audio-driven wave params: low-mids → amplitude, sub-bass → wavelength, centroid → speed
- Gentle music = slow rolling waves, aggressive = choppy
- Fake refraction: darken troughs, brighten crests via vertex color modulation (no extra render pass)
- ~900 vertices (30x30 grid), few sin() calls per frame — computationally trivial
- Ideal for smooth/ambient music (Enya, chillout, classical)
- Replaces hex grid as background in a "Calm" theme preset
- Could pair with softer cube materials (glass/ice) and slower camera

**Particle flow field (theme: Flow/Ambient):**
- Sea of particles following a 2D curl noise vector field — natural swirling currents
- Quiet music = smooth laminar drift, beats inject turbulence that settles back
- Audio mapping: sub-bass beats → field disruption (scatter), low-mids → eddy scale, centroid → color temp, flux → new eddies on transitions
- Technique: 3D curl noise — curl of a 3D scalar noise field gives divergence-free flow vectors. Particles swirl around and through the board in all three axes.
- Particles are persistent (no spawn/despawn, no lifetime) — just position, no velocity. Flow field provides direction each frame. Extremely cheap: N particles × 1 noise lookup per frame.
- 3D depth: particles behind board occluded by cubes (depth testing), particles in front float over — snow globe / aquarium feel.
- Could layer with water surface or stand alone as background
- Pairs with ambient, classical, chillout music

**Multi-stage firework shells (90s screensaver, high fidelity):**
- Three-stage lifecycle: launch → detonate → cascade. 6-10 seconds total per shell.
- **Launch:** single bright point arcs upward on a parabolic trajectory (3-5s). Leaves a slow-dissipating sparkle trail affected by gravity — builds anticipation.
- **Detonation:** at apex, shell explodes into 30-50 primary streamers radiating outward in parabolic arcs.
- **Cascade:** each primary streamer leaves secondary trail particles. Secondaries also gravity-affected, creating drooping curtain shapes. Everything fades over 3-5s.
- Only 1-2 active at a time — rarity makes each one an event.
- Trigger: spectral flux spikes (song transitions) or very strong bass beats. Once every 30-60s.
- Trail particles: gravity-affected, slow fade, long-lived. Key to the anticipation and afterglow feel.
- Could coexist with quick-burst fireworks (current) — shells are the rare dramatic moments, bursts are the frequent punctuation.
- Variation: different shell types (palm, willow, chrysanthemum) as theme options

**Particle cloud shapes (dissolve + reform):**
- A 3D shape (sphere, torus, icosahedron) defined as a point cloud — surface made of particles, not polygons. Spins on 3 axes in the background void.
- Particles drift loosely around their target positions (noise offset) — shape breathes, never perfectly solid.
- On musical trigger (flux spike, rare beat): shape dissolves — particles scatter with wind/gravity force. Slow, dramatic.
- After dissolve: new shape forms at a random position. Particles attracted toward new target positions. Slow coalescence from scattered cloud into recognizable form.
- Full lifecycle: formed (30-60s) → dissolve (3-5s) → scattered drift (5-10s) → reform into new shape (5-10s).
- Audio mapping: rotation speed = mids, drift amount = low-mids (tight when quiet, loose when loud), dissolve trigger = flux spike, reform speed = centroid (bright = fast crystallize, dark = slow coalesce).
- Shape vocabulary: sphere, torus, icosahedron, cube, double helix — rotates through shapes on each reform.
- Technique: per-particle target position + current position. Lerp toward target + noise. On dissolve: apply explosion force. On reform: assign new targets. Simple, no physics engine.
- Renders as small transparent quads in 3D scene pass — depth tested against board cubes.
- Slow burn ambient effect — perfect for idle/visualizer mode.

### Rendering Quality (to explore)
- **MSAA** — 4x multi-sample anti-aliasing (enabled, smooths geometry edges)
- **Supersampling** — render at 2x resolution, downsample. Smoothest possible but expensive.
- **Higher geometry detail** — rings 64+ segments, denser grids for smoother curves
- **Higher-res bloom** — finer bloom kernel at higher resolution for photographic quality
- **Thinner geometry at high res** — grid lines and cube gaps can be finer when pixels are smaller

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

**Beat detection overhaul (done — multi-band):**
- 7 independent detectors, one per FFT band
- Per-band beat events with decaying intensity
- Routed to different visual effects (bass→rings, upper-mids→particles, presence→grid)

**Beat band fingerprinting (new — dynamic beat routing):**
- Problem: we hardcode bass bands (0,1) as "the beat" for rings/sway/pulse. But some songs carry rhythm in mids (snare-driven), upper-mids (electronic), or presence (hi-hats). The visual beat misses the actual musical beat.
- Goal: detect which band(s) carry the most *rhythmic regularity* (not just energy) and assign those to the core beat visual effects.
- Approach: track per-band beat *regularity* over a settling window (~10s). The band with the most consistent inter-beat intervals is the rhythm carrier. Could be as simple as variance of beat gaps per band — low variance = regular = rhythmic.
- Route the identified "beat band" to: rings, board pulse, camera bass sway, bass zoom. These become the song's visual heartbeat.
- This could run as a cheap rolling computation alongside existing beat detection.
- Can also work as a one-time fingerprint pass on the first ~10s of a song, then lock in.
- Open: what happens during transitions where the beat shifts to a different band? Smooth crossfade or hard switch?
- Open: should there be a primary beat band + secondary? (kick + snare = two-layer rhythm)
- Open: can the FFT visualizer show which band is identified as the beat? (highlight or marker)
- This is the single most impactful audio-visual sync improvement remaining.

**Beat detection tuning:**
- Per-band threshold tuning — bass needs different sensitivity than highs
- Open: should beat events carry intensity, or just binary on/off?

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

**Done:**
- Effects interface: 6 AudioEffect modules + CameraReactor + EffectFlags (16 toggles) ✓
- Theme piece color overrides: `themed_piece_color()` with per-theme palettes ✓
- Theme switching: F1 cycles Default/Water/Debug with toast notification ✓
- Rolling averages + dominant band ranking ✓
- Dynamic audio-visual mapping: EffectBindings + SignalRank system ✓
- Two-phase analysis (7s initial lock, 30-45s resample) ✓
- Settings persistence (volume, theme, shuffle, music folder, window size) ✓
- Render state layer (BoardRenderState, GameStatusRender, HeldPieceRender) ✓

**Remaining:**
- More themes: fire, neon, minimal, etc.
- **Dynamic mapping refinements:**
  - Minimum energy threshold: bands below a threshold shouldn't be ranked. If only 3-4 bands are active, ranks draw from those only.
  - Active band count detection: rank 3 on a 3-band song = least active of active bands, not a dead band.
  - Track change detection: reset analysis when track changes (currently blends between songs).
  - Beat confidence tuning: fast guitar patterns can confuse the rhythmic band detector. Needs better distinction between rhythmic regularity and energetic activity.
- **Debug theme analysis dashboard:**
  - Visual display of all 7 band confidence values (bar chart or numeric)
  - Visual display of all 7 rolling energy values alongside
  - Show the ranked output: which bands are rank 1/2/3 for energy and separately for confidence
  - Highlight which band is "the beat" vs "most active" — they're separate lists
  - Allows listening to a song while verifying the algorithm is finding the foot-tap component
  - Enables more nuanced artistic ranking decisions for different effects
  - Stretch: interactive rank override — click to manually assign a band to a rank for A/B testing
- **Grid distortion workshop** — explore variations:
  - Per-band warp points: each frequency band drives its own distortion point at a fixed grid position
  - Piece-tracking warp: falling piece subtly pulls the grid like a rubber band as it passes through
  - Multiple gentle ripple points that interfere with each other
  - Warp intensity driven by band energy (gentle ambient undulation vs beat-driven pulses)
  - Currently on debug theme only — needs tuning before promoting to default/water
- Settings persistence (volume, shuffle state, selected theme survive restart) ✓
- **Cube material workshop** — needs interactive tuning, not recompile cycles:
  - Build a debug slider panel (separate window or overlay) for real-time adjustment of:
    - Front face alpha (tested 0.4), back face brightness (tested 0.25-0.5)
    - Fresnel per-face boost values, edge highlight intensity
    - Glow multiplier (tested 2.0-4.0), saturation/brightness curves
  - Jumping-off values from testing: front alpha=0.4, back dim=0.25-0.5, edge=0.08-0.15, fresnel sides=0.08-0.10
  - Currently reverted to candy colors (flat + subtle edge highlight) pending debug tooling
- 3D elements replacing 2D HUD overlays:
  - Shaped transport buttons (play triangle, pause bars, skip arrows)
  - Button press animation (depth halve on click)
  - Game over / pause as 3D elements
  - Score/level/lines as 3D floating text
- Responsive layout: side assemblies track window edges
- XDG-compliant config and data paths
- Linux packaging (Flatpak, Snap, AUR)
- Future: user-remappable key bindings, 3D in-game filesystem browser, multi-band beat tuning per-band thresholds

### Rendering Techniques Reference

**Currently using:**

| Technique | Status |
|---|---|
| Blinn-Phong lighting | Implemented |
| Per-face vertex color gradient | Implemented |
| Translucency (alpha blending) | Implemented |
| Per-band glow modulation | Implemented |
| Beat depth pulse | Implemented |
| Bloom post-processing | Implemented |
| MSAA 4x | Implemented |

**Tetris Effect techniques to consider (prioritized):**

| Technique | Impact | Effort | Notes |
|---|---|---|---|
| Schlick fresnel | Very high | Low | ~5 shader lines, `dot(normal, viewDir)`. Single biggest visual upgrade. |
| Rounded cube geometry | High | Moderate | Beveled edges catch specular highlights. Combined with fresnel = gem look. |
| Color grading / LUT | High | Low | Post-process color curve per theme. Instant cinematic feel. |
| HDR + tonemap | High | Moderate | Float color space, brighter highlights without wash-out. |
| Fake subsurface scattering | Medium | Low | Tint back faces with transmitted front color. Cheap inner glow. |
| Emissive per-pixel | Medium | Moderate | Different surface areas glow differently. Patterns, center vs edge. |
| Normal mapping | Medium | Moderate | Fake surface detail. Needs UV coords + texture. |
| SSAO | Medium | Hard | Darkens creases where cubes meet. Depth and grounding. |
| Environment mapping | Medium | Hard | Cubes reflect surroundings (cubemap or SSR). |
| Soft particles | Low | Moderate | Particles fade near surfaces instead of hard-clip. |
| Depth of field | Low | Hard | Background blur, focus on play area. |
| Motion blur | Low | Hard | Fast pieces leave streaks. |
| Volumetric lighting | Low | Hard | God rays, light shafts. |

---

*This plan will be refined as open questions are resolved and architecture takes shape.*
