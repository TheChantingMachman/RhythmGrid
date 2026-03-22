# Audio-Visual Catalog

Signal inventory and visual effects registry for RhythmGrid's audio-reactive system.

## Audio Signals

### Band Energy (7 channels)
Real-time FFT energy per frequency band. Available in three variants:

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
- `bands[i]` — raw energy (0.0-1.0, absolute)
- `bands_norm[i]` — normalized to own recent peak (0.0-1.0, relative to recent activity)
- `peak_bands[i]` — slow-decay peak hold (visual indicator use)

### Band Beat (7 channels)
Per-band spike detection. Binary trigger with decaying intensity.

- `band_beat_intensity[i]` — 1.0 on beat, decays at 4.0/sec
- Independent per band: bass can beat without highs and vice versa
- Threshold: energy > rolling_mean × 1.5, min gap 0.3s per band

### Spectral Centroid (planned)
Weighted center of mass of the spectrum. Single float, roughly 0.0-1.0.
- Low value = dark/warm sound (energy concentrated in bass)
- High value = bright/sharp sound (energy concentrated in highs)
- Shifts during verse → chorus, builds, breakdowns
- Pipeline spec needed: cheap computation on existing FFT data

### Spectral Flux (planned)
Rate of change in the spectrum between consecutive frames. Single float ≥ 0.0.
- High value = sound is changing rapidly (transients, note attacks, transitions)
- Low value = sustained or static sound (pads, held notes, silence)
- Useful for detecting musical transitions without band-specific beats
- Pipeline spec needed: diff between consecutive FFT frames

### Danger Level (modifier)
Game-state driven escalation signal (0.0-1.0). Not audio-derived but modulates audio-visual effects:
- 0.0 = normal play, stack is low
- 1.0 = danger zone, stack near top
- Used to intensify existing effects, not as a primary signal

---

## Visual Effects Catalog

### Ring Spawner
**Current assignment:** band_beat[0] (sub-bass), band_beat[1] (bass)
**Behavior:** Spawns expanding concentric ring on beat trigger. Ring grows from radius 0.5 to 18.0, fading with squared falloff over 2-3 seconds. Color tinted by danger level.
**Parameters:** max_radius, lifetime, color, alpha

### Particle Burst
**Current assignment:** band_beat[4] (upper-mids), band_beat[5] (presence)
**Behavior:** Spawns 24-40 small particles from board edges on beat. Particles fly outward with gravity and drag.
**Parameters:** count, speed, lifetime, color, size

### Line Clear Particles
**Current assignment:** game event (line clear)
**Behavior:** 120 tiny particles spray across cleared row area. Fast dissipation (0.5-1.1s).
**Parameters:** count, spread, lifetime, color, size

### Clearing Cell Flash
**Current assignment:** game event (line clear)
**Behavior:** White cubes shrink from full size to zero over 0.4s at each cleared cell position.
**Parameters:** duration, color, scale curve

### Cube Glow
**Current assignment:** bands_norm[type_index % 7]
**Behavior:** Each tetromino type's brightness modulated by its assigned band. Glow multiplier 0-2.0 drives saturation and brightness in push_cube_3d.
**Parameters:** glow_multiplier, saturation_range, brightness_range

### Grid Line Shimmer
**Current assignment:** beat_intensity (legacy) + bands_norm[5] (presence)
**Behavior:** Grid line RGB brightens on beat and presence energy. Additive color boost.
**Parameters:** boost_per_beat, boost_per_band, base_color

### Hex Dot Breathing
**Current assignment:** bands_norm[2] (low-mids) for size, bands_norm[0] (sub-bass) for color
**Behavior:** Background hex dots grow/shrink with low-mids energy. Color shifts warm on sub-bass.
**Parameters:** min_size, max_size, color_shift_range

### Hex Line Alpha
**Current assignment:** bands_norm[0] + bands_norm[2] via geo_alpha
**Behavior:** Connecting lines between hex dots brighten with low-mid and sub-bass energy.
**Parameters:** base_alpha, energy_multiplier

### Camera Bass Sway
**Current assignment:** band_beat[0] (sub-bass), band_beat[1] (bass)
**Behavior:** Slow sinusoidal horizontal camera drift on bass beats. Amplitude scales with danger.
**Parameters:** sway_amplitude, frequency, danger_scaling

### Camera Hi-Freq Jitter
**Current assignment:** band_beat[5] (presence), band_beat[6] (brilliance)
**Behavior:** Rapid micro-offset on X and Y from high-frequency beats. Different frequencies for X and Y to avoid repetition.
**Parameters:** x_amplitude, y_amplitude, x_frequency, y_frequency

### Impact Shake
**Current assignment:** game event (line clear, hard drop)
**Behavior:** Decaying high-frequency camera shake. Intensity proportional to lines cleared.
**Parameters:** initial_intensity, decay_rate, x_amplitude, y_amplitude, oscillation_speed

### T-Spin Flash
**Current assignment:** game event (t-spin detected)
**Behavior:** Large magenta "T-SPIN" text fades over 1 second at screen center.
**Parameters:** duration, color, scale, position

### Combo Counter
**Current assignment:** game state (combo_count > 0)
**Behavior:** Gold "COMBO N" text visible during active combo streak.
**Parameters:** color, position, scale

### Level-Up Burst
**Current assignment:** game event (level change)
**Behavior:** 3 expanding cyan rings + 120 particles from board center.
**Parameters:** ring_count, ring_color, particle_count, particle_color

### Hex Field Rotation
**Current assignment:** time + danger_level modifier
**Behavior:** Background hex grid rotates. Speed increases with danger level.
**Parameters:** base_speed, danger_multiplier

---

## Unassigned / Future Effects

Effects to build, not yet implemented:

- **Board Pulse** — cubes briefly scale up on their band's beat (depth/size modulation)
- **Color Temperature** — scene-wide color shift based on spectral centroid (warm↔cool)
- **Spectral Transition Flash** — brief visual event when spectral flux spikes (song section change)
- **Bass Zoom** — camera Z nudges forward on heavy bass hits
- **Trail/Afterimage** — ghost trail behind falling piece that pulses with mids
- **Board Edge Glow** — edge of the game board emits light colored by dominant band

---

## Migration Status

**Migrated to AudioEffect trait** (src/gui/effects/):
- BeatRings — expanding rings on bass beats
- HexBackground — rotating dot grid + connecting lines
- FftVisualizer — 7-band bars with peak hold + lock
- GridLines — wireframe with centroid color + presence shimmer

**Migrated to CameraReactor** (src/gui/camera.rs):
- Bass sway, hi-freq jitter, bass zoom, impact shake

**Remaining inline** (game-state dependent — need expanded RenderContext):
- Cube glow + depth pulse (needs Grid + piece type)
- Active piece glow (needs ActivePiece)
- Ghost piece (needs ActivePiece + Grid)
- Clearing cell flash (needs clearing_cells Vec)
- Particles (externally triggered — beat pulse + line clear + level-up)

**Not audio effects** (UI controls, stay inline):
- Volume bar, transport buttons, folder button
- HUD text (score, level, lines, combo, t-spin, track name)
- Preview pieces (next + held)
- Game over / pause overlays

## Design Notes

**AudioEffect trait** works well for purely audio-driven effects. Effects cache audio values in update() and render geometry in render(). Camera lives in its own CameraReactor.

**Boundary:** Game-state-dependent effects (cubes, pieces, clearing) need Grid/ActivePiece access that AudioFrame + RenderContext don't carry. Options:
- Expand RenderContext with game state references (couples effects to game types)
- Keep them inline (pragmatic — they're not swappable per-theme anyway)
- New trait with broader context (over-engineering for now)

**Open questions:**
- Should bindings be static per theme, or shift dynamically based on dominant band ranking?
- How many simultaneous effects before visual noise overwhelms? Need a budget.
- How to handle game-state-dependent effects in the theme system?
