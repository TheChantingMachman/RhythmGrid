# Audio-Visual Catalog

Signal inventory, visual effects registry, and EffectFlags mapping for RhythmGrid's audio-reactive system.

## Audio Signals

### Band Energy (7 channels)
Real-time FFT energy per frequency band:

| Band | Index | Range | Musical Content |
|---|---|---|---|
| Sub-bass | 0 | 20-60Hz | Kick drums, bass synth fundamentals |
| Bass | 1 | 61-250Hz | Bass guitar, bass synth harmonics |
| Low-mids | 2 | 251-500Hz | Vocals (low), guitar body, warmth |
| Mids | 3 | 501-2kHz | Vocals (core), guitar, piano, snare body |
| Upper-mids | 4 | 2-4kHz | Vocal presence, snare crack, guitar bite |
| Presence | 5 | 4-8kHz | Hi-hats, cymbal shimmer, vocal sibilance |
| Brilliance | 6 | 8-20kHz | Air, sparkle, cymbal decay |

**Variants per band:**
- `bands[i]` — raw energy (0.0-1.0)
- `bands_norm[i]` — normalized to own recent peak (0.0-1.0)
- `peak_bands[i]` — slow-decay peak hold (visual indicator)
- `band_beat_intensity[i]` — 1.0 on beat, decays at 4.0/sec

### Spectral Centroid
Weighted center of mass of the spectrum. `centroid` float 0.0-1.0.
- Low = dark/warm (energy in bass). High = bright/sharp (energy in highs).
- Drives grid line color temperature (warm↔cool shift).

### Spectral Flux
Rate of spectral change between frames. `flux` float ≥ 0.0.
- High = transients, note attacks, song transitions.
- Drives hex background brightness and future transition effects.

### Danger Level (modifier)
Game-state escalation (0.0-1.0). Not audio-derived. Modulates effect intensity:
- Camera sway amplitude, hex rotation speed, ring color shift.

---

## EffectFlags → Visual Effects Mapping

Every effect is toggleable per-theme via `EffectFlags`. Three presets: Default (all on except hex), Water (all on except fireworks), Debug (all off).

### Migrated Effect Modules (src/gui/effects/)

| Flag | Module | Signals | What it renders |
|---|---|---|---|
| `beat_rings` | BeatRings | band_beats[0,1], bands_norm, danger | Expanding concentric rings on sub-bass/bass beats |
| `hex_background` | HexBackground | bands_norm[0,2], flux, danger | Rotating hex dot grid + connecting lines |
| `grid_lines` | GridLines | band_beats (max all), bands_norm[5], centroid | Wireframe with centroid color temp + presence thickness pulse |
| `fft_visualizer` | FftVisualizer | bands, peak_bands, hud_opacity | 7-band bars + peak hold indicators + lock toggle |
| `fireworks` | Fireworks | band_beats (all), bands_norm | Radial spark bursts from random positions, color by band |
| `camera_sway` | CameraReactor | band_beats[0,1,5,6], danger | Bass sway, hi-freq jitter, smooth bass zoom |

### Inline Scene Effects (src/gui/scene.rs)

| Flag | Signals | What it renders |
|---|---|---|
| `cube_glow` | bands_norm[type%7], band_beat_intensity[type%7] | Per-piece-type glow + beat depth pulse on occupied cubes |
| `ghost_piece` | — (pure game state) | Translucent ghost at drop position |
| `active_piece_pulse` | bands_norm[type%7], band_beat_intensity[type%7] | Active piece glow + beat depth pulse |
| `clearing_flash` | — (game event) | White shrinking cubes at cleared cells over 0.4s |
| `t_spin_flash` | — (game event) | Magenta "T-SPIN" text fade at screen center |
| `level_up_rings` | — (game event) | 3 expanding cyan rings + particle burst on level change |
| `combo_text` | — (game state) | Gold "COMBO N" text during active streak |

### Inline World Effects (src/gui/world.rs)

| Flag | Signals | What it triggers |
|---|---|---|
| `particle_beat_pulse` | band_beat_intensity[4,5] | 40-100 particles from board edges on upper-mid beats |
| `line_clear_particles` | — (game event) | 120 particles per cleared line |
| `camera_shake` | — (game event) | Decaying shake on line clear / hard drop |

---

## Camera (src/gui/camera.rs)

CameraReactor handles all camera audio reactivity. Guarded by `camera_sway` flag. `camera_shake` has its own flag (event-driven, not audio-driven).

| Behavior | Signal | Description |
|---|---|---|
| Bass sway | band_beats[0,1] | Slow horizontal drift, amplitude scales with danger |
| Hi-freq jitter | band_beats[5,6] | Rapid micro-offset on X/Y |
| Smooth bass zoom | band_beats[0,1] | Camera Z pushes forward, ease-in fast / ease-out slow |
| Impact shake | game event | Decaying oscillation on line clear, intensity = lines cleared |

---

## Theme Presets

| Theme | Piece Colors | Effects Off | Camera Feel |
|---|---|---|---|
| Default | Pipeline standard | hex_background | Punchy sway, strong zoom |
| Water | Blue gradient (7 shades) | fireworks | Gentle drift, minimal zoom |
| Debug | Pipeline standard | Everything | Static camera |

---

## Roadmap: Future Signals & Effects

### Signals to add
- **Beat fingerprinting** — identify THE rhythmic band per song, route to beat effects dynamically
- **Rolling energy ranking** — dominant band detection over 5-10 second window
- **Onset strength** — more sophisticated transient detection than threshold-based beats

### Effects to build
- **Grid distortion** — wireframe bends toward beat sources (Geometry Wars style)
- **Particle trails** — ribbon geometry connecting particle history positions
- **Water surface** — displaced vertex grid driven by audio wave params
- **Curl noise flow** — 3D particle flow field with audio-driven turbulence
- **Firework shells** — multi-stage launch → burst → cascade (3-5 second lifecycle)
- **Point cloud shapes** — 3D form made of particles, dissolves and reforms
- **Board edge glow** — edge emits light colored by dominant band

### Cube material improvements
- Semi-transparent (done, 75%/85% alpha)
- Per-face gradient (done, darker centers / brighter edges)
- Fix transparent overlap (sort or alpha-to-coverage)
- Edge glow / fake fresnel
- Shader-based real fresnel
- Per-vertex material modulation (specular, emissive per band)
- Rounded edges
- Metallic material variant
- Bitmap-extruded blocks (pixel art silhouettes)
