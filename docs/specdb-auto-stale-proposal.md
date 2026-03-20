# SpecDB Proposal: Auto-Stale on `modifies` Dependencies

## Problem

When a new spec declares `modifies: [SomeType]` and depends on implemented specs that define `SomeType`, the human author must manually mark those dependencies as stale. If they forget, the pipeline builds the new spec against stale tests that assert pre-change behavior, causing failures that neither the Coding Agent nor the Test Agent can resolve.

This has happened twice:
- **Build 39:** `config.music_folder` extended `Settings` struct. `config.load_save` tests constructed `Settings` with struct literals → missing field error.
- **Build 62+:** `game.lock_delay` changed `tick()` behavior. `game.tick` tests expected immediate `PieceLocked` → now returns `Nothing` with a timer.

Both were the same pattern: the `modifies` field flagged the risk, but the human didn't manually stale the affected dependency.

## Proposed Solution

### Auto-stale at build time

When the pipeline begins building a spec that has `modifies: [X, Y]`:

1. For each item in `modifies`, find which `depends_on` entries define that item
2. If any of those entries are `implemented`, automatically flip them to `stale`
3. This triggers refactor mode for the affected entries — the Spec Diff Agent audits their tests and the Test Agent updates them

### Why at build time, not at `specdb add`?

- At add time, the spec is `draft` — the author might still be iterating on it
- Auto-staling at add would trigger unnecessary pipeline work for WIP specs
- The build trigger is the commitment point — that's when tests actually need updating

## CLI Changes

### `specdb validate` — Improved Warning (Current Behavior Enhanced)

Current output:
```json
{"type":"type_breakage_risk","entry_id":"game.lock_delay","modifies":["GameSession","tick"],"implemented_deps":["game.lock","game.move_down","game.tick"]}
```

Proposed output (human-readable mode, e.g. `--format human`):
```
⚠ TYPE BREAKAGE RISK: game.lock_delay

  This entry modifies: GameSession, tick

  These implemented dependencies define modified items and may
  need refactoring:

    game.tick (implemented) — defines 'tick'
      Tests assert pre-lock-delay behavior. Implementing
      game.lock_delay will change what tick() returns.

    game.lock (implemented) — defines lock behavior
      Tests may construct GameSession without lock_delay fields.

  BEFORE BUILDING, ask yourself:
    "Does implementing this entry change what an existing
     function RETURNS or DOES in certain cases?"

  If YES (behavior change), mark affected specs stale:
    specdb update --id game.tick --status stale

  If NO (only type/struct extension), the pipeline's
  type_breakage_risk handling may be sufficient.

  After the auto-stale upgrade, this will be handled
  automatically at build time.
```

### `specdb add/update --modifies` — Inline Guidance

When adding or updating a spec with `--modifies`:

```
ℹ game.lock_delay modifies [GameSession, tick]

  Implemented dependencies that may be affected:
    game.tick — defines 'tick' (IMPLEMENTED)
    game.lock — related to lock behavior (IMPLEMENTED)

  If this entry changes the BEHAVIOR of these functions
  (not just their types), mark them stale:
    specdb update --id game.tick --status stale

  Type changes (struct fields, enum variants) are caught
  automatically. Behavior changes require manual stale.
```

### After Auto-Stale Upgrade

#### `specdb add/update --modifies` output:

```
ℹ game.lock_delay modifies [GameSession, tick]

  Implemented dependencies that will be auto-staled at build:
    game.tick — defines 'tick' (will become stale)

  No manual action needed. The pipeline will refactor
  affected tests when this entry is built.
```

#### Build-time output (pipeline integration):

```
ℹ AUTO-STALE: Preparing to build game.lock_delay

  modifies: [GameSession, tick]

  Auto-staling affected dependencies:
    game.tick: implemented → stale
      Reason: defines 'tick', which game.lock_delay modifies
      Effect: Pipeline will enter refactor mode for game.tick
              and update its tests before building lock_delay

  Proceeding with chunked refactor build...
```

#### `specdb validate` output (after upgrade):

```
ℹ game.lock_delay modifies [GameSession, tick]

  Dependency game.tick (implemented) will be auto-staled
  at build time. No action needed.
```

## Implementation Notes

### Matching `modifies` items to specs

The `modifies` field contains type/function names (e.g., `Settings`, `tick`, `GameSession`). To match these to specs:

1. **src-map lookup:** Check which files define these types/functions, then find which specs map to those files
2. **depends_on intersection:** Only consider specs that are already in the `depends_on` graph — a modified type in an unrelated spec is not affected
3. **Conservative approach:** If a `modifies` item can't be matched to a specific spec, emit a warning but don't auto-stale

### Edge cases

- **Multiple specs modify the same type:** Each should independently trigger stale on the defining spec. Order doesn't matter — stale is idempotent.
- **Circular modifies:** Spec A modifies type from spec B, spec B modifies type from spec A. Both go stale. Pipeline handles them in dependency order.
- **modifies without depends_on:** The modifies item isn't in any dependency. Emit warning: "modifies [X] but no dependency defines X — is depends_on complete?"

### Backward compatibility

- Auto-stale is additive — no existing behavior changes
- Specs without `modifies` are unaffected
- The `type_breakage_risk` warning remains for specs where auto-stale hasn't been triggered yet (e.g., validation before build)

## Summary

| Aspect | Current | After Upgrade |
|--------|---------|---------------|
| Risk detection | `validate` emits JSON warning | Same, plus human-readable guidance |
| Stale action | Manual by human author | Automatic at build time |
| Failure mode | Pipeline build fails, requires human intervention | Pipeline auto-refactors affected tests |
| Author burden | Must remember to stale dependencies | Zero — just set `modifies` correctly |
| CLI guidance | Minimal JSON output | Contextual help with exact commands |
