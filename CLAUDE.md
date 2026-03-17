# RhythmGrid

Audio-reactive puzzle game that turns your music library into a dynamic visual experience.

## Dark Factory Pipeline

The pipeline is test-first agentic. The flow is: **spec → pipeline builds tests → pipeline builds source → tests pass.** We manage the spec only — do not hand-write tests or source implementations for pipeline-owned domains.

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

## SpecDB Usage

Binary: `./specdb-linux-amd64` (auto-discovers `spec/spec.yaml` from repo root)

### Common Commands

```bash
# Query
specdb query --status draft --format brief          # overview of unbuilt work
specdb query --tags-all "store,write" --format brief # entries matching ALL tags (AND)
specdb query --search "expir" --status draft         # substring search within status
specdb query --rdeps some.id --format brief          # reverse deps (exclusive flag)

# Add
specdb add --id cmds.ping --section Commands \
  --description "PING returns PONG" --tags "core,ping"
specdb add --id cmds.mget --section Commands \
  --description "MGET returns multiple values" \
  --tags "core,read" --depends-on cmds.get

# Update
specdb update --id cmds.ping --status implemented
specdb update --id cmds.ping --constants "timeout=30"

# Remove / Rename
specdb remove --id cmds.ping                          # draft only
specdb remove --id cmds.ping --force --status-override # non-draft
specdb rename --id cmds.old_name --new-id cmds.new_name

# Validation & Diffing
specdb validate
specdb validate --test-dir tests
specdb snapshot                    # copy spec.yaml → .spec-snapshot.yaml
specdb diff                       # changes since last snapshot
specdb impact --test-dir tests    # affected entries + test files
```

### Gotchas

- `--tags` is OR (any match). Use `--tags-all` for AND (all must match).
- `--rdeps` is exclusive — don't combine with `--status`/`--tags`/etc.
- `--format brief` has no header row. Count = number of lines.
- `--constants` values are always strings (`timeout=30` → `"30"`).
- `remove` is permanent. Prefer: `specdb update --id X --status deprecated`
- `remove` blocks non-draft entries unless `--status-override` is given.
- `rename` updates `depends_on` refs but not test files or source code.
- Lifecycle: `draft → implemented → stale → deprecated`
