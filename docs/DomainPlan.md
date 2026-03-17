# RhythmGrid — Domain Plan

*Tentative split of work between the Dark Factory pipeline and co-authored development.*

## Pipeline Domains (spec → test → implement)

Behavior can be clearly specified and verified by automated tests.

### Audio Engine
- Audio file decoding (MP3, WAV, FLAC, OGG → PCM samples)
- Beat detection and BPM estimation
- FFT / frequency band decomposition (bass, mids, highs)
- Energy and amplitude tracking per frame

### Game Logic
- Grid data model (state, rules, transitions)
- Puzzle mechanics (player action + grid state → new grid state)
- Difficulty scaling algorithm
- Score/progress tracking (if applicable)

### Infrastructure
- Audio file discovery and playlist building
- Config and settings parsing (load, save, validate)
- File format validation

## Co-Authored Domains (human + AI collaboration)

Require creative judgment, real-time feedback, or cross-cutting design decisions.

### Visual / Feel
- Rendering pipeline and shader work
- Animation design and tuning
- Audio-to-visual mapping (which frequencies drive which effects)
- Color palette selection and theming
- Particle effects appearance and tuning

### Design
- Core puzzle mechanic (TBD — the central "what does the player do?" question)
- UX flow and onboarding
- Session structure and pacing

### Architecture
- Overall module structure and how pipeline-built components wire together
- Real-time event/data flow between audio analysis and rendering
- Platform and distribution decisions

## Gray Zone

Algorithmic logic is pipeline-testable, but tuning requires human evaluation.

- Color palette system — logic testable, palette choices are aesthetic
- Particle effects — spawn/physics logic testable, appearance is not
- Difficulty scaling — algorithm testable, "feel" needs playtesting
- Audio-to-grid sensitivity — thresholds are tunable constants, but good values need ears and eyes

---

*This plan is tentative. Boundaries will shift as the core puzzle mechanic is defined and architecture takes shape.*
