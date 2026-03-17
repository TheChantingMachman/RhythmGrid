# RhythmGrid

Audio-reactive puzzle game that turns your music library into a dynamic visual experience.

## Dark Factory Pipeline

The pipeline is test-first agentic:
1. We write specs (via SpecDB CLI or editing `spec/spec.yaml` directly)
2. Pipeline agent builds tests from specs
3. Pipeline agent builds source to pass those tests

**Do not hand-write tests or source for pipeline-owned domains.** We manage the spec only. See `docs/DomainPlan.md` for what the pipeline owns vs. what we co-author.

## Branching

- Develop on `dev`. Merge into `main` for major versions only.
- Default working branch is `dev`. Do not commit directly to `main`.

## Documentation Hygiene

The `docs/` folder contains planning and design documents. As the project grows, be mindful of context window usage:
- Don't read entire docs unless needed — use targeted reads for the relevant section.
- When docs become stale or redundant with what's in the code/specs, prune them.
- Prefer the spec (`spec/spec.yaml`) as the source of truth for implemented behavior. Docs are for planning and decisions, not restating what the code already says.
- If a doc section has been fully resolved and absorbed into code/specs, consider removing it or collapsing it to a one-liner summary.

## Project Structure

- `src/` — Rust source code
- `tests/` — test files
- `spec/` — SpecDB spec files
- `docs/` — product planning and documentation
- `df-config.json` — Dark Factory pipeline config

## SpecDB

Binary: `./specdb-linux-amd64` — run with no args for full help. Auto-discovers `spec/spec.yaml`.

Key rules:
- `spec.yaml` is a **flat YAML list** with `- id:` items. Do NOT nest under section keys.
- Prefer the CLI (`specdb add`, `specdb update`) over hand-editing YAML.
- Run `specdb validate` to catch format errors.
- Lifecycle: `draft → implemented → stale → deprecated`
