# RhythmGrid

Audio-reactive puzzle game that turns your music library into a dynamic visual experience.

## Roles

### Dark Factory Pipeline (autonomous agent)
Test-first agentic builder:
1. We write specs (via SpecDB CLI or editing `spec/spec.yaml` directly)
2. Pipeline agent builds tests from specs
3. Pipeline agent builds source to pass those tests

**Do not hand-write tests or source for pipeline-owned domains.** The pipeline owns all headless/logic domains: grid, pieces, game logic, audio engine, music management, config/infrastructure. It can also own **testable GUI logic** — coordinate mapping, input→action mapping, render state derivation, color schemes, layout math, animation state machines — anything where behavior can be verified without a real screen.

### Us (Claude + human, co-authored)
We own:
- **Spec authorship** — writing and managing `spec/spec.yaml`
- **GUI scaffolding** — windowing, render pipeline setup, game loop (wgpu/winit/cpal wiring)
- **Render glue** — thin code that takes pipeline-built render state and issues actual draw calls
- **Visual/Feel tuning** — shaders, particle appearance, color palette aesthetics, animation polish
- **Smoke testing** — verifying the integrated experience works end-to-end
- **Architecture decisions** — module structure, event flow, platform choices, src-map maintenance
- **Design** — core mechanic, UX flow, session pacing

### Guiding Principle
Maximize what the pipeline builds by speccing testable logic layers beneath GUI features. Our hand-written code should be a thin shell over pipeline-built logic.

## Branching

- Develop on `dev`. Merge into `main` for major versions only.
- Default working branch is `dev`. Do not commit directly to `main`.
- Co-authored GUI/rendering work: develop on `gui/rendering`, squash-merge to `dev` at milestones (no PR needed). Pull `dev` into `gui/rendering` to pick up pipeline builds.

## Documentation Hygiene

The `docs/` folder contains planning and design documents. As the project grows, be mindful of context window usage:
- Don't read entire docs unless needed — use targeted reads for the relevant section.
- When docs become stale or redundant with what's in the code/specs, prune them.
- Prefer the spec (`spec/spec.yaml`) as the source of truth for implemented behavior. Docs are for planning and decisions, not restating what the code already says.
- If a doc section has been fully resolved and absorbed into code/specs, consider removing it or collapsing it to a one-liner summary.

## Project Structure

- `src/` — Pipeline-owned Rust library code (game logic, audio, render state, input mapping)
- `src/gui/` — **Co-authored** GUI code (wgpu renderer, windowing, visual effects, theme). Pipeline must not modify these files. Excluded via `df-config.json` `src_exclude`.
- `src/main.rs` — Thin launcher, also excluded from pipeline.
- `tests/` — Pipeline-owned test files
- `spec/` — SpecDB spec files
- `docs/` — Product planning and documentation
- `df-config.json` — Dark Factory pipeline config

## SpecDB

Binary: `./specdb-linux-amd64` — run with no args for full help. Auto-discovers `spec/spec.yaml`.

Key rules:
- `spec.yaml` is a **flat YAML list** with `- id:` items. Do NOT nest under section keys.
- Prefer the CLI (`specdb add`, `specdb update`) over hand-editing YAML.
- Run `specdb validate` to catch format errors.
- Lifecycle: `draft → implemented → stale → deprecated`
- **Auto-stale:** Updating any non-status field on an `implemented` entry automatically flips it to `stale`, triggering a pipeline rebuild. For cosmetic edits that don't need a rebuild, pass `--status implemented` in the same update call.
- **`modifies` field:** Declares which existing spec entries this spec will affect. Takes **spec IDs** (not type names): `--modifies config.load_save,game.tick`. Triggers three validate checks:
  - `dangling_modifies` — hard error if modifies ID doesn't exist
  - `unlinked_modifies` — warning if modifies ID not in `depends_on`
  - `type_breakage_risk` — warning if modified spec is `implemented`
- **When to use `modifies`:** Ask: "will implementing this draft change behavior or types that another spec's tests assert?" If yes, add `--modifies <that-spec-id>`. Common cases: adding struct fields, changing function return values, altering function behavior.
- **Auto-stale:** The pipeline auto-stales `implemented` entries that appear in a building spec's `modifies` field. No manual stale needed — just set `modifies` correctly and the pipeline handles the rest.
- **Inline hints (v2.9.0+):** The CLI will emit stderr hints at add/update time if modifies IDs are not in depends_on or if they're implemented. Current v2.8.0 stores modifies as opaque strings — the enhanced validation lands with v2.9.0.
