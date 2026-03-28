# RhythmGrid — Product Plan

## Vision

Audio-reactive Tetris driven by the player's own music library. Classic gameplay wrapped in a synesthetic experience where everything pulses, shifts, and evolves with the music. Doubles as a music player and visualizer in idle mode.

## Target Audience

- **Casual competitive players** — want satisfying Tetris with audiovisual flair
- **Audiophiles** — want a music player/visualizer that happens to be a game
- **Linux gamers** (primary), broader platform reach as a goal

## Release Tiers

1. **Personal use** — move fast, no licensing constraints
2. **Linux distribution** — Flathub/Snap/AUR. Requires clean asset licensing, XDG paths, PipeWire/PulseAudio
3. **Steam** — keep feasible (no platform lock-in), defer until relevant

## Tech Stack

| Layer | Choice |
|---|---|
| Language | Rust |
| Rendering | wgpu (Vulkan/Metal/DX12) |
| Windowing | winit |
| Audio | cpal + symphonia |

## Current State (v0.2.0)

**Gameplay:** 7-bag SRS, hold piece, ghost piece, T-spin detection, combos, lock delay (400ms/15 resets), Guideline gravity curve, demo mode.

**Audio:** 7-band FFT, multi-band beat detection, spectral centroid + flux, per-band normalization, dynamic band ranking, streaming decode, shuffle mode.

**Visuals:** 9 themes (Default, Water, Space, Flow, Fluid, Crystal, Fractal, Pipes, Debug). OIT transparency, GGX specular, bloom, MSAA 4x, GPU compute (Flow particles, Mandelbrot zoom). Theme auto-cycle (4 min).

**Renderer:** wgpu with persistent GPU buffers, PostProcessChain, GpuEffect trait, deduplicated WGSL lighting.

See [archive/CompletedWork.md](archive/CompletedWork.md) for full feature inventory.

---

## Priority Backlog

### Completed
- ~~Credits screen~~ — done. Full CC BY 3.0 attribution, accessible from title menu.
- ~~Visual juice (Phase A)~~ — done. Z-shake on hard drop, smoother trails, board wave on tetris.
- ~~Title screen~~ — done. Play/Settings/Credits/Exit, keyboard nav, theme backgrounds, idle fade.
- ~~Demo AI~~ — done. Greedy placer with scoring (height, holes, clears, bumpiness).
- ~~Action audio feedback (Phase A)~~ — done. Music-amplification EQ boost via ActionAudioProcessor. Swappable architecture with enabled flag. Gets ~80% of the way to proper feel.
- ~~Journey mode~~ — done. 8 themes escalate by coolness every 25 lines, F1 unlocks at 200.
- ~~Vanish zone collision fix~~ — done. Column bounds checked in vanish zone.

### High Priority

*No high priority items currently. Next impactful work is in medium/low priority.*

### Medium Priority

**Playlists** — save/load named playlists, queue management, drag-to-reorder. Currently folder-only with sequential/shuffle play.

**Controller support** — gamepad input via gilrs (MIT/Apache-2.0, license-clean) + DAS/ARR tuning. Deprioritized for now.

### Low Priority

**5-6 next piece preview** — bag/peek infrastructure supports it. Must integrate with the minimalist UI — consider subtle translucent stack or compact vertical strip. Avoid cluttering the scene.

**Settings menu** — dedicated screen for volume, DAS/ARR, key bindings, visual options. Currently buried in pause overlay.

**Folder drag-and-drop** — drop a folder onto the window to set music library.

**Repeat mode** — repeat-one / repeat-all toggle alongside existing shuffle.

**3D font / SDF text** — replace the 3x5 bitmap font with proper text rendering. Options: SDF font atlas (smooth at any scale), or 3D extruded letter meshes for the title screen. Current bitmap font is functional but visibly pixelated at large scales (title screen).

**Expand built-in music library** — more CC/royalty-free tracks. Organize in `assets/music/` with a manifest (artist, title, license, source URL). OGG Vorbis preferred for size.

**Action audio feedback tuning** — the EQ boost approach (ActionAudioProcessor) gets ~80% there but needs further work:
- Tune ramp rate, decay curves, and per-action intensity levels
- Experiment with simple non-invasive SFX as an alternative or complement — quiet mechanical taps/clicks that don't conflict harmonically with any genre
- Explore SFX sets matched to song profile: percussive set for electronic, soft clicks for acoustic/classical, pitched tones for melodic tracks. Could auto-select based on spectral centroid or beat confidence.
- The ActionAudioProcessor is swappable — new approaches can be prototyped without rearchitecting. Settings toggle (enabled flag) ready for when settings menu ships.

---

## Action-Reactive Audio (the "Feel Gap")

The core Tetris Effect differentiator: every player action produces musical sound. The game becomes an instrument. Hardest gap to close but highest-impact for the "wow" factor.

### What Tetris Effect does

1. **Rotate** — melodic note pitched to current key/scale
2. **Move** — subtle percussive tick
3. **Soft drop** — accelerating tone
4. **Hard drop** — bass impact hit + screen shake
5. **Lock** — satisfying click/thud
6. **Line clear** — crescendo sweep proportional to lines cleared
7. **T-spin** — distinctive power-up sound
8. **Combo** — ascending pitch sequence
9. **Level up** — fanfare or transition sound
10. **All sounds quantized to beat** — actions snap to nearest musical subdivision

### Design philosophy

Tetris Effect's SFX work because they were composed *as part of the music* — the stems have gaps for the player to fill. We're the inverse: the player's music is the star. Any SFX we add is an uninvited guest in someone else's song. A bright rotate chime that works over ambient electronica will clash over a jazz piano trio.

### Phased approach

**Phase A — Music-amplification feedback (our approach):**
Instead of adding foreign sounds, amplify the music itself at the moment of player action. On a hard drop, briefly boost the dominant frequency band for 30-50ms. On a rotate during a hi-hat, swell that range. The game doesn't add sounds — it turns a spotlight on what's already playing. Sidesteps the audiophile objection entirely: it's *their* music, momentarily more vivid.

- Per-band EQ boost on the music, aligned to whatever's active at the moment of action
- Builds entirely on existing per-band energy analysis
- Self-regulating: boosting silence is still silence, busy music gets richer feedback
- Hard drop → sub-bass/bass swell. Rotate → upper-mid boost. Clear → full-spectrum bloom.

**Phase A fallback — quiet mechanical taps:**
When the music is too quiet to boost meaningfully (below an energy threshold), fill in with extremely quiet unpitched percussive sounds — soft clicks and taps, not melodic notes. Think mechanical keyboard, not musical instrument. No pitch means no harmonic conflict with any genre. Mixed low enough to be felt more than heard.

The louder the music, the less the game adds. The quieter the music, the more the taps fill in. The game adapts to the music's density.

**Visual juice (ships with Phase A):**
- Camera Z-shake on hard drop (no lateral — causes motion sickness)
- Line clear flash (extend existing T-spin flash to all clears)
- Piece lock particle burst
- Combo counter scale-up animation
- Board edge glow pulse on beat

**Phase B — Beat-quantized boost (medium effort):**
- Time the EQ boosts to snap to the nearest beat subdivision (1/8 or 1/16)
- Action lands slightly early? Delay the boost 20ms to hit the grid. Player doesn't notice, but it feels "in time."
- Uses existing beat detection + a tempo tracker for the beat grid
- Latency must stay imperceptible (<50ms snap window)

**Phase C — Key-matched synthesis (hard, stretch goal):**
- Detect current musical key from FFT (chromagram analysis)
- Generate pitched action tones from that scale (pentatonic fallback for safety)
- Blend with the EQ boost: the music swells AND a harmonized note layers in
- Full Tetris Effect territory — but with arbitrary music, not curated stems

---

## Beta Release Checklist

### GPU Acceleration Audit
- Profile 262K particle dispatches — actual GPU time, bottleneck on integrated GPUs
- Test on Intel/AMD integrated graphics — wgpu Vulkan backend may need fallbacks
- WGSL shader compilation stall on first frame (shader cache warm-up)
- 16MB particle buffer on VRAM-constrained systems
- Theme auto-cycle GPU teardown/reinit — VRAM leaks over long sessions
- CPU fallback path (gpu: None) — verify it's a viable degraded mode
- Mandelbrot f32 precision — cap zoom gracefully
- Graceful degradation: what happens when GPU compute isn't available?

### General Efficiency Audit
- Profile tick() hot path (audio analysis + effects + phase machines)
- Per-frame allocations (Vec::new, String formatting in render paths)
- OIT at high particle counts — verify no banding
- PostProcessChain resize — verify no texture leaks
- Audio decode memory over long sessions (hours)
- Settings save on every theme cycle — debounce for SSD wear
- active_count.max() means GPU processes MAX_PARTICLES forever after first wrap — consider periodic compaction

### Legal / Licensing
- Decide project license: MIT or Apache-2.0 (see Guidelines.md — unresolved since project start)
- Symphonia (audio decoder) is MPL-2.0 — file-level copyleft. Evaluate whether a pure MIT/Apache alternative exists before public distribution. It's the one license outlier in the stack.
- Full dependency license audit (`cargo license` or manual) — verify no GPL/LGPL crept in via transitive deps
- Built-in music attribution — ✓ done (credits screen + assets/LICENSES.md)

### Platform Testing
- Vulkan on AMD/NVIDIA/Intel Linux
- Windows: test on actual hardware (not just cross-compile)
- Wayland vs X11 edge cases
- PipeWire / PulseAudio compatibility
- High-DPI / fractional scaling

---

## Roadmap

### Next up — near-term
1. **Journey system polish (WIP)** — current implementation works but needs:
   - *Transition effect/fanfare* — theme changes are abrupt. Add a visual transition (flash, fade, particle burst) when advancing to the next stage. Should feel like an event, not a glitch.
   - *Auto-registration of new themes* — current JOURNEY_ORDER is a hardcoded array that must be manually updated when themes are added. Refactor so new themes auto-register with a coolness rank, and the journey builds the order dynamically. Eliminates maintenance.
   - *Pro player scaling* — 200 lines is trivial for experienced players. Consider: journey length scales with game speed setting, or journey milestones are level-based not line-based, or journey is a percentage of the player's historical average game length. Default should never feel like a gate to someone who clears 200 lines in 5 minutes.
   - *Settings exposure* — future settings menu should allow: journey length (100/200/500/endless), disable journey (free theme switching from start), lock to a single theme. The current defaults should be good for casual players without configuration.
2. **Playlists** — save/load/reorder, track metadata
3. **Controller support** — gamepad via gilrs + DAS/ARR (deprioritized)

### Medium-term — polish
4. **Settings menu**
5. **5-6 next preview** — minimalist, respects clean UI
6. **Expand built-in music library**
7. **Repeat mode**
8. **Folder drag-and-drop**
9. **3D font / SDF text**
10. **Action audio feedback tuning** — further iteration on EQ boost + SFX experiments

### Long-term — differentiation
10. **Advanced visualizer AI** — music-aware demo that paces to the beat, chases tetrises during high-energy sections, times clears to strong beats
11. **Adaptive theme selection** — auto-switch based on music energy
12. **Background environments** — procedural 3D scenes per theme
13. **Screen-space effects** — chromatic aberration, vignette, radial blur
14. **GPU particle generalization** — extend compute shader to all effects
15. **Visualizer-only mode** — fullscreen music visualization, no game

### Horizon — "rivals Tetris Effect"
16. **Beat-quantized boost (Phase B)** — snap EQ boosts to beat subdivisions via tempo tracker
17. **Key-matched synthesis (Phase C)** — chromagram key detection, pentatonic action tones blended with boost
18. **Custom music analysis** — pre-scan for BPM, sections, energy map
19. **Platform expansion** — web (WebGPU), macOS, Steam

---

## The Hard Gap: Music-Reactive Gameplay Audio

Items 19-22 above. Don't attempt until Phase A feedback is shipped and validated.

### Why this is hard for RhythmGrid

We play *arbitrary user music*, not curated stems. Three unsolved problems:

**Problem 1 — Key detection:**
Detecting the current musical key from FFT in real time is an active research area. Simple chromagram approaches achieve ~70-80% on clean pop but fail on atonal/ambient, key changes, complex harmony, detuned synths. Wrong notes are more jarring than no notes.

*Fallback:* pentatonic scale. Minor pentatonic (5 notes) is consonant over almost any western music. Skip key detection, play from fixed pentatonic transposed to FFT's dominant pitch class. Trades precision for safety.

**Problem 2 — Beat grid alignment:**
Our beat detection gives beat *events* (this frame had a beat), not a beat *grid* (the next beat arrives in 127ms). Quantization requires predicting the future.

Building a beat grid requires: BPM estimation (tempo tracking), phase alignment, tempo stability assessment, handling variable tempos.

*Fallback:* retrospective snapping. Snap to the last detected beat with a fixed subdivision. Works well for steady-tempo music, degrades gracefully (no snap) for irregular tempos.

**Problem 3 — Arrangement integration:**
Tetris Effect fades musical stems in/out. We can't do this with a mixed-down audio file.

*Fallback:* sidechain ducking. When an action sound plays, music volume dips 20-30% for 50-100ms. Simple (multiply output buffer by smoothed envelope), makes action sounds cut through any mix. Stretch: spectral ducking (only lower conflicting frequencies).

### Implementation order

Each problem is independent. Each builds on the music-amplification foundation from Phase A:

1. **Phase B (beat-quantized boost):** tempo tracker -> beat grid -> snap EQ boosts to subdivisions. The music swells *on the beat* even if the player acted slightly off-beat.
2. **Phase C (key-matched synthesis):** chromagram -> dominant pitch class -> pentatonic tones blended with the EQ boost. The music swells AND a harmonized note layers in.
3. **Phase D (arrangement integration):** spectral ducking during the fallback taps (only lower conflicting frequencies). Makes the percussive taps sit *inside* the mix rather than on top.

Phase B without C = rhythmically satisfying boosts. Phase C without B = harmonically matched tones. Both together: the game amplifies and harmonizes with any song. Add D and the fallback taps disappear into the mix.

### Technical prerequisites
- **Per-band EQ manipulation:** apply gain per frequency band to the audio output buffer in real time (Phase A). We have per-band energy — need per-band gain applied to the output PCM.
- **Audio mixer:** fallback tap sounds mixed into cpal output alongside music (Phase A). Currently audio thread only plays music — needs multi-source mixer.
- **Tempo tracker:** rolling BPM estimation from beat events (Phase B)
- **Chromagram:** 12-bin pitch class energy from FFT (Phase C). Straightforward transform of existing 7-band FFT.
- **Audio synthesis:** generate pitched tones at specific frequencies (Phase C)

### Honest framing
Even with all phases, we differ from Tetris Effect fundamentally: they have *authored* experiences (specific songs, hand-tuned stems). We have *generative* experiences (any music, algorithmic analysis). Ours will never feel as choreographed as their best levels. But ours works with the player's own library — that's the trade.

Our approach has a unique advantage: music-amplification feedback gets *better* with better music. A player's favorite song will feel more alive in RhythmGrid than in Tetris Effect, because we're enhancing what they already love rather than playing over it with pre-authored stems.

---

## Ideas Backlog

These are future possibilities, not commitments. Explore when relevant.

- **Adaptive theme selection** — auto-switch based on rolling energy + centroid + beat confidence
- **Melodic drift signal** — slow spectral average (5-20s timescale) to steer starfield direction, aurora flow, camera orbit
- **Music-reactive demo AI** — pace to music, time clears to beats, aggression tiers
- **Dynamic point lights** — firework/clear flashes cast momentary colored light onto nearby cubes
- **Grid distortion** — vertex displacement by force fields (per-band warp, piece tracking, interfering ripples)
- **Particle trails/ribbons** — ribbon of quads connecting particle position history
- **Cube material workshop** — debug slider panel for real-time material tuning
- **Bitmap-extruded blocks** — pixel art silhouettes extruded into mini-voxel columns
- **Particle cloud shapes** — dissolve/reform cycle on musical triggers
- **Water surface background** — sine-wave vertex displacement, audio-driven wave params

---

*Completed work and historical design notes: [archive/CompletedWork.md](archive/CompletedWork.md)*
