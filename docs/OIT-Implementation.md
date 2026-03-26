# Weighted Blended OIT — Implementation Plan

Temporary design doc for the `feature/weighted-oit` branch.
Delete this file when the feature merges to dev.

## Problem

Transparent geometry (board pieces at alpha 0.75, rotating preview pieces, effects) has z-ordering bugs because the transparent render pass reads depth but doesn't write it. Draw order determines which pixel wins, causing:
- Preview piece faces drawing over each other incorrectly during rotation
- Board bottom-row top faces drawing over front faces of row above (cabinet angle)
- Any overlapping transparent geometry relying on submission order

## Solution

Replace the transparent render pass with McGuire & Bavoil 2013 Weighted Blended OIT. Transparent fragments accumulate into two render targets using depth-weighted blending. A compositing pass combines the result over the opaque scene. No sorting needed.

## Render Pass Sequence

```
Pass 1: Opaque         — depth write ON, color write ALL (unchanged)
Pass 2: OIT Accumulate — depth read-only, two color targets (accum + revealage)
Pass 3: OIT Composite  — fullscreen quad, alpha blend over opaque scene
Pass 4: HUD            — no depth (unchanged)
Pass 5: Bloom          — post-process (unchanged)
```

## New GPU Resources

| Resource | Format | MSAA | Purpose |
|----------|--------|------|---------|
| oit_accum_msaa | Rgba16Float | 4x | Accumulation render target |
| oit_accum_resolve | Rgba16Float | 1x | Resolved accum for composite read |
| oit_reveal_msaa | R8Unorm | 4x | Revealage render target |
| oit_reveal_resolve | R8Unorm | 1x | Resolved revealage for composite read |
| oit_composite_bgl | — | — | Bind group layout for composite shader |

## New Pipelines

### OIT Accumulation Pipeline
- **Vertex shader**: same `vs_main`
- **Fragment shader**: new `fs_oit` — runs existing lighting, then outputs:
  - Target 0 (accum): `vec4(color.rgb * alpha * w, alpha * w)` — additive blend (One + One)
  - Target 1 (revealage): `alpha` — multiplicative blend (Zero + OneMinusSrc), clear to 1.0
- **Depth**: read-only, compare Less, write OFF
- **MSAA**: 4x (must match all color attachments)

### OIT Composite Pipeline
- **Shader**: fullscreen triangle, reads resolved accum + revealage textures
- **Output**: `vec4(avg_color, 1.0 - revealage)` with standard alpha blending
- **Renders to**: msaa_texture -> scene_texture (LoadOp::Load preserves opaque)
- **No depth test**

## Weight Function

```wgsl
let d = clip_position.z;  // 0..1 in wgpu (0=near, 1=far)
let w = clamp(alpha * max(1e-2, 3e3 * pow(1.0 - d * 0.99, 3.0)), 1e-2, 3e3);
```

May need tuning — camera at z=16, geometry at z~0, NDC z values clustered near 1.0.

## Blend States

### Accumulation target (RGBA16Float)
```
color: One + One (Add)
alpha: One + One (Add)
clear: (0, 0, 0, 0)
```

### Revealage target (R8Unorm)
```
color: Zero + OneMinusSrc (Add)
alpha: Zero + OneMinusSrc (Add)
clear: (1, 1, 1, 1)
```

### Composite output
```
Standard alpha blending: SrcAlpha + OneMinusSrcAlpha
```

## Implementation Steps

### 1. New textures + resize handling
Add 4 texture views to GpuState. Create helper methods following existing pattern.
Update `resize()` to recreate them.

### 2. OIT accumulation shader
Add `fs_oit` entry point. Reuses existing lighting code from `fs_main`.
Outputs to two targets with depth-based weight.
Keep `discard` for soft particles (safe in OIT).

### 3. OIT accumulation pipeline
Two color targets with specific blend states (see above).
Depth read-only from opaque pass.

### 4. OIT composite shader + pipeline + bind group
Fullscreen triangle reads resolved textures, outputs premultiplied result.
New bind group layout for the two OIT textures + sampler.

### 5. Update render() pass execution
Replace Pass 2 (transparent) with OIT accumulate.
Insert Pass 3 (OIT composite) before HUD.
Keep render() signature unchanged (opaque, transparent, hud).

### 6. Remove old transparent pipeline
Delete `scene_pipeline_transparent` from struct and construction.

### 7. Simplify scene.rs
Remove sorting/culling from `render_preview_piece`.
Remove depth pre-pass buffers if present.
Revert to 3-tuple return from `build_scene_and_hud`.

## Edge Cases

- **MSAA**: all color attachments in OIT pass must be 4x. Resolve before composite reads.
- **Soft particles**: `discard` is safe — discarded fragments don't accumulate.
- **Ghost pieces** (alpha 0.08): very low weight, correct behavior — faint overlay.
- **R8Unorm precision**: sufficient for ~20 overlapping layers at alpha 0.75.
- **wgpu BlendFactor::OneMinusSrc**: available since wgpu 0.14+, we're on wgpu 24.

## Files Changed

| File | Scope |
|------|-------|
| src/gui/renderer.rs | Major — new textures, pipelines, shaders, render passes |
| src/gui/scene.rs | Minor — simplify render_preview_piece, revert to 3-tuple |
| src/gui/app.rs | None (signature unchanged) |
| src/gui/drawing.rs | None |
