# RhythmGrid — Project Guidelines

## Core Principle

Own everything, rent nothing. No dependency, asset, or decision should expose the project to legal risk, restrict where it can be deployed, or create obligations to third parties.

## Licensing

### Project License
- RhythmGrid source code: **MIT or Apache-2.0** (decide before first public release)
- You own it, others can use it, no legal exposure from your own licensing choice

### Dependency Licenses — Allowed
- MIT
- Apache-2.0
- BSD (2-clause, 3-clause)
- Zlib
- CC0 / public domain (for assets)
- OFL (for fonts)

### Dependency Licenses — Not Allowed
- **GPL / LGPL** — copyleft forces your code open or imposes linking constraints
- **Proprietary / commercial** — restricts distribution
- **SSPL / Commons Clause** — fake-open licenses with commercial restrictions
- **Any license with patent retaliation clauses** that could limit deployment

### Audit Rule
Every crate and asset must have its license verified before being added to the project. When in doubt, don't use it.

## Intellectual Property

### The Tetris Situation
- "Tetris" is a trademark of The Tetris Company. **Never use the word "Tetris" in the game, store listings, descriptions, or marketing.**
- Game mechanics (falling pieces, line clearing, grid) are **not copyrightable** — settled law.
- Tetromino shapes are geometric and not copyrightable.
- SRS (Super Rotation System) is community-documented, not patented.
- **Do not replicate** Tetris brand colors, official sound effects, or recognizable trade dress.
- The name "RhythmGrid" is clean — keep it that way.

### Assets
- All bundled assets (audio, fonts, icons, images) must be:
  - Self-made, OR
  - CC0 / public domain, OR
  - OFL (fonts only), OR
  - Explicitly licensed for redistribution with no restrictions
- No "free for personal use" assets — must be clear for commercial distribution.
- Procedural generation is always preferred where feasible (audio, textures, effects).

### Trademarks
- Do not reference other game names, brands, or trademarked terms in any user-facing context.
- Internal docs and planning can reference inspiration (e.g., "Tetris Effect inspired") but nothing shipped should.

## Dependencies

### Philosophy
- Prefer pure Rust crates — no C bindings or system library wrappers where avoidable.
- Fewer dependencies > more dependencies. Only add what's needed.
- No SDK lock-in — platform-specific integrations (e.g., Steamworks) must be behind feature flags, never in the core binary.
- No runtime services or network dependencies for core functionality.

### Current Approved Stack
| Crate | Purpose | License |
|---|---|---|
| wgpu | Rendering | MIT/Apache-2.0 |
| winit | Windowing | Apache-2.0 |
| cpal | Audio output | Apache-2.0 |
| symphonia | Audio decoding | MPL-2.0* |
| bytemuck | GPU struct casting | MIT/Apache-2.0 |
| pollster | Async block_on | MIT/Apache-2.0 |
| rfd | Native file dialogs | MIT |
| rustfft | FFT analysis | MIT/Apache-2.0 |
| serde + toml | Settings serialization | MIT/Apache-2.0 |

*\* **symphonia note:** MPL-2.0 is file-level copyleft — modified symphonia source files must stay MPL-2.0, but it does not infect your own code. Generally considered safe for use in proprietary/permissive projects. However, before any public release, revisit whether a pure MIT/Apache-2.0 decoder alternative exists. Don't forget this — it's the one license outlier in the stack.*

## Platform & Distribution

- Core binary must have no hard platform-specific API dependencies.
- Linux is primary, but don't preclude cross-platform.
- XDG conventions for config and data paths.
- Single binary or simple package — no complex runtime dependencies.
- Store-specific integrations (Steam, Flathub metadata, etc.) are additive, never required.

## General Attitude

- When choosing between two options, pick the one with less legal surface area.
- If a dependency or asset has a complicated or ambiguous license, skip it.
- Criticism is fine. Lawsuits are not. Optimize accordingly.
