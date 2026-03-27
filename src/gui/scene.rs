// Scene building — constructs 3D geometry and 2D HUD from game state.
// Separated from world.rs to keep rendering logic isolated from game logic.

use rhythm_grid::game::GameState;
use rhythm_grid::grid::{CellState, WIDTH, HEIGHT};


use super::drawing::*;
use super::theme::*;
use super::world::GameWorld;

/// Embossed text — light highlight up-left, darker text on top. Looks raised/carved out.
fn push_text_embossed(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                      x: f32, y: f32, text: &str, color: [f32; 4], scale: f32) {
    let off = scale * 1.2; // wider offset for cleaner separation
    // Subtle light highlight (up-left)
    let highlight = [
        (color[0] + 0.2).min(1.0),
        (color[1] + 0.2).min(1.0),
        (color[2] + 0.2).min(1.0),
        color[3] * 0.3,
    ];
    push_text(verts, indices, x - off * 0.3, y - off, text, highlight, scale);
    // Main text on top
    push_text(verts, indices, x, y, text, color, scale);
}

/// Render a tetromino piece in world space using push_cube_3d, with 3-axis rotation.
/// OIT handles transparent face ordering — no sorting or culling needed.
fn render_preview_piece(
    verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
    cells: &[(i32, i32)], color: [f32; 4], depth: f32, glow_boost: f32,
    world_pos: [f32; 3], angles: [f32; 3],
) {
    if cells.is_empty() { return; }

    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for &(dr, dc) in cells {
        cx += dc as f32 + 0.5;
        cy += dr as f32 + 0.5;
    }
    cx /= cells.len() as f32;
    cy /= cells.len() as f32;

    let start_vert = verts.len();

    for &(dr, dc) in cells {
        let col = dc as f32 - cx;
        let row = dr as f32 - cy;
        push_cube_3d(verts, indices, col, row, depth, color, glow_boost, 0, 0.0);
    }

    // 3-axis rotation + translation
    let (sx, cx_r) = (angles[0].sin(), angles[0].cos());
    let (sy, cy_r) = (angles[1].sin(), angles[1].cos());
    let (sz, cz) = (angles[2].sin(), angles[2].cos());

    for v in &mut verts[start_vert..] {
        let p = v.position;
        let y1 = p[1] * cx_r - p[2] * sx;
        let z1 = p[1] * sx + p[2] * cx_r;
        let x2 = p[0] * cy_r + z1 * sy;
        let z2 = -p[0] * sy + z1 * cy_r;
        let x3 = x2 * cz - y1 * sz;
        let y3 = x2 * sz + y1 * cz;
        v.position = [x3 + world_pos[0], y3 + world_pos[1], z2 + world_pos[2]];

        let n = v.normal;
        let ny1 = n[1] * cx_r - n[2] * sx;
        let nz1 = n[1] * sx + n[2] * cx_r;
        let nx2 = n[0] * cy_r + nz1 * sy;
        let nz2 = -n[0] * sy + nz1 * cy_r;
        let nx3 = nx2 * cz - ny1 * sz;
        let ny3 = nx2 * sz + ny1 * cz;
        v.normal = [nx3, ny3, nz2];
    }
}

/// Build 3D scene (world-space cubes, background) and 2D HUD (NDC overlay)
/// Returns (opaque_scene, transparent_scene, hud) geometry.
pub fn build_scene_and_hud(world: &GameWorld) -> ((Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>)) {
    let cube_depth = 0.75; // chunkier cubes for more substantial 3D feel

    let mut sv = Vec::new(); // opaque scene
    let mut si = Vec::new();
    let mut tv = Vec::new(); // transparent scene
    let mut ti = Vec::new();

    let gw = WIDTH as f32;
    let gh = HEIGHT as f32;

    // Background effects + hex/beat_rings + level-up rings
    {
        let fx_ctx = super::effects::RenderContext {
            board_width: gw, board_height: gh,
            win_w: THEME.win_w as f32, win_h: THEME.win_h as f32,
            window_aspect: world.window_aspect,
            preview_angle: world.preview_angle,
            hud_opacity: world.hud_opacity,
        };
        world.effects.render_dashboard(&mut tv, &mut ti, &fx_ctx);
        world.effects.render_background(&mut tv, &mut ti, &fx_ctx);
    }
    // Level-up rings (animation-driven, not an effect module)
    build_level_up_rings(&mut tv, &mut ti, world, gw, gh);

    // Occupied cells — glow per piece type, pulse from dynamic rank analysis
    let ef = &world.effects.flags;
    let pulse_band = world.resolve_rank(world.bindings.board_pulse);
    let glow_band = world.resolve_rank(world.bindings.cube_glow);
    for cell in &world.render_board.occupied {
        let mut color = rgba_to_f32(world.themed_piece_color(cell.type_index));
        color[3] = 0.75;
        let (band_glow, depth) = if ef.cube_glow {
            (world.analysis.bands_norm[glow_band] * 1.5,
             cube_depth + world.analysis.band_beat_intensity[pulse_band] * 0.22)
        } else {
            (0.0, cube_depth)
        };
        // Contact AO: check grid neighbors (1=up, 2=down, 4=left, 8=right)
        let r = cell.row as usize;
        let c = cell.col as usize;
        let g = &world.session.grid.cells;
        let mut nb = 0u8;
        if r > 0 && g[r-1][c] != CellState::Empty { nb |= 1; }
        if r + 1 < HEIGHT && g[r+1][c] != CellState::Empty { nb |= 2; }
        if c > 0 && g[r][c-1] != CellState::Empty { nb |= 4; }
        if c + 1 < WIDTH && g[r][c+1] != CellState::Empty { nb |= 8; }
        // Check settle animation for this cell
        let settle = world.anims.settle_cells.iter()
            .find(|s| s.col == cell.col && s.row == cell.row)
            .map(|s| (s.timer / super::animations::SETTLE_DURATION).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        push_cube_3d(&mut tv, &mut ti, cell.col as f32, cell.row as f32, depth, color, band_glow, nb, settle);
    }

    // Ghost piece
    if ef.ghost_piece {
        for cell in &world.render_board.ghost {
            let base_color = world.themed_piece_color(cell.type_index);
            let ghost_color = rgba_to_f32([base_color[0], base_color[1], base_color[2], 40]);
            push_cube_3d(&mut tv, &mut ti, cell.col as f32, cell.row as f32, cube_depth * 0.2, ghost_color, 0.0, 0, 0.0);
        }
    }

    // Active piece
    for cell in &world.render_board.active {
        let band = (cell.type_index as usize) % 7;
        let mut color = rgba_to_f32(world.themed_piece_color(cell.type_index));
        color[3] = 0.85;
        let (active_glow, active_depth) = if ef.active_piece_pulse {
            (world.analysis.bands_norm[band] * 1.5, cube_depth + world.analysis.band_beat_intensity[band] * 0.22)
        } else {
            (0.0, cube_depth)
        };
        push_cube_3d(&mut tv, &mut ti, cell.col as f32, cell.row as f32, active_depth, color, active_glow, 0, 0.0);
    }

    // Next piece preview (world-space, right of board)
    {
        let next_color = rgba_to_f32(world.themed_piece_color(world.render_next.type_index));
        let band = (world.render_next.type_index as usize) % 7;
        let glow = if ef.cube_glow { world.analysis.bands_norm[band] * 1.0 } else { 0.0 };
        render_preview_piece(
            &mut tv, &mut ti,
            &world.render_next.cells, next_color, cube_depth, glow,
            [13.0, -2.5, 0.0],
            [world.preview_angle * 0.3, world.preview_angle * 0.7, world.preview_angle * 0.15],
        );
    }

    // Held piece preview (world-space, left of board)
    if let Some(ref held) = world.render_held {
        let held_color = rgba_to_f32(world.themed_piece_color(held.type_index));
        let band = (held.type_index as usize) % 7;
        let glow = if ef.cube_glow { world.analysis.bands_norm[band] * 1.0 } else { 0.0 };
        render_preview_piece(
            &mut tv, &mut ti,
            &held.cells, held_color, cube_depth, glow,
            [-3.0, -2.5, 0.0],
            [world.preview_angle * 0.2, world.preview_angle * 0.5, 0.0],
        );
    }

    // Hard drop trails — translucent streaks from start to landing
    for trail in &world.anims.drop_trails {
        let progress = 1.0 - (trail.timer / super::animations::DROP_TRAIL_DURATION).max(0.0);
        let alpha = (1.0 - progress) * 0.35;
        let mut color = rgba_to_f32(world.themed_piece_color(trail.type_index));
        color[3] = alpha;
        // Cap trail length to 6 rows near the landing point
        let trail_start = trail.start_row.max(trail.end_row - 6);
        for row in trail_start..trail.end_row {
            if row >= 0 && row < HEIGHT as i32 {
                let fade = (row - trail_start) as f32 / (trail.end_row - trail_start).max(1) as f32;
                let mut c = color;
                c[3] = alpha * fade; // brightest at landing, fades upward
                push_cube_3d(&mut tv, &mut ti, trail.col as f32, row as f32, cube_depth * 0.15, c, 0.0, 0, 0.0);
            }
        }
    }

    // Subtle fall ghost — faint echo one row behind the active piece (level 5+)
    if world.session.state == GameState::Playing && world.render_status.level >= 5 {
        for cell in &world.render_board.active {
            if cell.row > 0 {
                let mut color = rgba_to_f32(world.themed_piece_color(cell.type_index));
                color[3] = 0.08;
                push_cube_3d(&mut tv, &mut ti, cell.col as f32, (cell.row - 1) as f32, cube_depth * 0.1, color, 0.0, 0, 0.0);
            }
        }
    }

    // Grid lines (effect module — renders to opaque pass)
    {
        let grid_ctx = super::effects::RenderContext {
            board_width: gw, board_height: gh,
            win_w: THEME.win_w as f32, win_h: THEME.win_h as f32,
            window_aspect: world.window_aspect,
            preview_angle: world.preview_angle,
            hud_opacity: world.hud_opacity,
        };
        world.effects.render_grid(&mut sv, &mut si, &grid_ctx);
    }

    // 3D Music Dashboard (volume, transport, folder, FFT visualizer)
    super::dashboard::build_dashboard(&mut tv, &mut ti, world, gw, gh);

    // Shatter fragments — soft glowing particles from line clears
    if ef.clearing_flash {
        let normal = [0.0f32, 0.0, 1.0];
        for frag in &world.anims.shatter_fragments {
            let life = (frag.timer / frag.max_life).clamp(0.0, 1.0);
            let alpha = life * life;
            if alpha < 0.01 { continue; }
            let s = frag.size * (0.5 + life * 0.5);
            let color = [frag.color[0], frag.color[1], frag.color[2], alpha * frag.color[3]];
            let z = 0.3;
            // Soft circle billboard with UV radial falloff
            let base = tv.len() as u32;
            tv.push(Vertex { position: [frag.x - s, -frag.y - s, z], normal, color, uv: [-1.0, -1.0] });
            tv.push(Vertex { position: [frag.x + s, -frag.y - s, z], normal, color, uv: [ 1.0, -1.0] });
            tv.push(Vertex { position: [frag.x + s, -frag.y + s, z], normal, color, uv: [ 1.0,  1.0] });
            tv.push(Vertex { position: [frag.x - s, -frag.y + s, z], normal, color, uv: [-1.0,  1.0] });
            ti.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    // HUD
    let (hv, hi) = build_hud(world);

    ((sv, si), (tv, ti), (hv, hi))
}

/// Background geometric field: hex grid + connecting web + beat rings
fn build_level_up_rings(sv: &mut Vec<Vertex>, si: &mut Vec<u32>, world: &GameWorld, gw: f32, gh: f32) {
    if !world.effects.flags.level_up_rings { return; }
    let ring_cx = gw / 2.0;
    let ring_cy = -gh / 2.0;
    let ring_z = -1.0;
    let ring_n = [0.0f32, 0.0, 1.0];
    let ring_segments = 32;

    for ring in &world.anims.bg_rings {
        let progress = 1.0 - ring.life / ring.max_life;
        let alpha = ring.color[3] * (1.0 - progress).powi(2);
        if alpha < 0.005 { continue; }
        let inner_r = ring.radius;
        let outer_r = ring.radius + 0.3 + (1.0 - progress) * 0.5;
        let color_inner = [ring.color[0], ring.color[1], ring.color[2], alpha];
        let color_outer = [ring.color[0] * 0.5, ring.color[1] * 0.5, ring.color[2] * 0.5, 0.0];

        for i in 0..ring_segments {
            let a0 = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / ring_segments as f32) * std::f32::consts::TAU;
            let (c0, s0) = (a0.cos(), a0.sin());
            let (c1, s1) = (a1.cos(), a1.sin());

            let base = sv.len() as u32;
            sv.push(Vertex { position: [ring_cx + c0 * inner_r, ring_cy + s0 * inner_r, ring_z], normal: ring_n, color: color_inner, uv: [0.0, 0.0] });
            sv.push(Vertex { position: [ring_cx + c1 * inner_r, ring_cy + s1 * inner_r, ring_z], normal: ring_n, color: color_inner, uv: [0.0, 0.0] });
            sv.push(Vertex { position: [ring_cx + c1 * outer_r, ring_cy + s1 * outer_r, ring_z], normal: ring_n, color: color_outer, uv: [0.0, 0.0] });
            sv.push(Vertex { position: [ring_cx + c0 * outer_r, ring_cy + s0 * outer_r, ring_z], normal: ring_n, color: color_outer, uv: [0.0, 0.0] });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }
}


/// HUD overlay in screen space
fn build_hud(world: &GameWorld) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut indices = Vec::new();
    let ef = &world.effects.flags;
    let t = &THEME;
    let w = t.win_w as f32;
    let h = t.win_h as f32;
    let text_col = rgba_to_f32(t.text_color);
    let dim_col = rgba_to_f32(t.dim_color);

    // --- Fadeable HUD elements (affected by hud_opacity) ---

    // Hold label (top-left)
    push_text(&mut verts, &mut indices, 12.0, 12.0, "HOLD", dim_col, 1.0);

    // Score / Level / Lines (left side, below held piece area)
    let stats_y = 110.0;
    push_text(&mut verts, &mut indices, 12.0, stats_y, "SCORE", dim_col, 1.0);
    let rs = &world.render_status;
    push_text(&mut verts, &mut indices, 12.0, stats_y + 12.0, &format!("{}", rs.score), text_col, 2.0);

    push_text(&mut verts, &mut indices, 12.0, stats_y + 38.0, "LEVEL", dim_col, 1.0);
    push_text(&mut verts, &mut indices, 12.0, stats_y + 50.0, &format!("{}", rs.level), text_col, 2.0);

    push_text(&mut verts, &mut indices, 12.0, stats_y + 76.0, "LINES", dim_col, 1.0);
    push_text(&mut verts, &mut indices, 12.0, stats_y + 88.0, &format!("{}", rs.total_lines), text_col, 2.0);

    // T-spin flash
    if ef.t_spin_flash && world.anims.t_spin_flash > 0.01 {
        let ta = (world.anims.t_spin_flash * 255.0) as u8;
        push_text(&mut verts, &mut indices, w / 2.0 - 40.0, h / 2.0 - 60.0,
                  "T-SPIN", rgba_to_f32([255, 100, 255, ta]), 3.0);
    }

    // Combo counter rendered after HUD fade (always visible)



    // Track queue display — positioned above audio controls
    // Use VolDown button screen rect as anchor for alignment
    let vol_rect = world.btn_rect(super::world::ButtonId::VolDown);
    let track_x = vol_rect[0] * (w / world.window_size[0]);
    let track_bottom = vol_rect[1] * (h / world.window_size[1]) - 8.0; // 8px above vol buttons
    if let Ok(audio) = world.audio.try_lock() {
        let list = &audio.track_list;
        let idx = audio.current_track_index;
        let num_shown = 4.min(list.len()); // now playing + next 3
        if !list.is_empty() {
            let track_top = track_bottom - num_shown as f32 * 10.0;
            // Now playing (highlighted)
            let now: String = list.get(idx).map(|s| s.chars().take(16).collect()).unwrap_or_default();
            push_text(&mut verts, &mut indices, track_x, track_top, &now.to_uppercase(), text_col, 1.0);
            // Next tracks
            for i in 1..num_shown {
                let next_idx = (idx + i) % list.len();
                let name: String = list[next_idx].chars().take(16).collect();
                push_text(&mut verts, &mut indices, track_x, track_top + i as f32 * 10.0,
                          &name.to_uppercase(), dim_col, 1.0);
            }
        } else if !audio.track_name.is_empty() {
            let display: String = audio.track_name.chars().take(16).collect();
            push_text(&mut verts, &mut indices, track_x, track_bottom - 10.0, &display.to_uppercase(), dim_col, 1.0);
        }
    }
    // Projected button labels
    let scale_x = w / world.window_size[0];
    let scale_y = h / world.window_size[1];

    // Helper to project a button label below its 3D cube
    let project_label = |id: super::world::ButtonId, world: &super::world::GameWorld| -> (f32, f32) {
        let [bx, by, bw, bh] = world.btn_rect(id);
        let lx = bx * scale_x + bw * scale_x * 0.5 - 4.0;
        let ly = (by + bh) * scale_y + 4.0;
        (lx, ly)
    };

    // Vol -/+ labels removed — 3D glyphs rendered in scene pass

    // Transport labels removed — 3D icons rendered on button faces in scene pass
    // Shuffle state indicator (text below shuffle button)
    let is_shuffled_lbl = if let Ok(audio) = world.audio.try_lock() { audio.shuffled } else { false };
    if is_shuffled_lbl {
        let (sx, sy) = project_label(super::world::ButtonId::Shuffle, world);
        push_text(&mut verts, &mut indices, sx - 2.0, sy, "ON", rgba_to_f32([100, 200, 255, 255]), 1.0);
    }

    // Folder label removed — 3D glyph rendered in scene pass

    // FFT lock label
    let lock_col = if world.fft_locked { text_col } else { dim_col };
    let (ll_x, ll_y) = project_label(super::world::ButtonId::FftLock, world);
    push_text(&mut verts, &mut indices, ll_x - 4.0, ll_y, "LOCK", lock_col, 1.0);

    // State overlays
    if rs.state == GameState::GameOver {
        push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([120, 0, 0, 80]), 0.08);
        let go_w = 400.0; let go_h = 300.0;
        let go_x = (w - go_w) / 2.0;
        let go_y = (h - go_h) / 2.0;
        push_panel(&mut verts, &mut indices, go_x, go_y, go_w, go_h, 0.09);
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 16.0, "GAME OVER", rgba_to_f32([255, 80, 80, 255]), 4.0);
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 60.0, "SCORE", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 84.0,
                  &format!("{}", rs.score), text_col, 5.0);
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 140.0,
                  &format!("LVL {}  LINES {}", rs.level, rs.total_lines), dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 166.0,
                  &format!("COMBO {}  PCS {}", rs.max_combo, rs.pieces_placed), dim_col, 2.0);
        let mins = (rs.time_played_secs / 60.0) as u32;
        let secs = (rs.time_played_secs % 60.0) as u32;
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 192.0,
                  &format!("TIME {}:{:02}", mins, secs), dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, go_x + 20.0, go_y + 240.0, "ENTER TO RESTART", dim_col, 2.0);
    }

    if rs.state == GameState::Paused {
        push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([0, 0, 0, 60]), 0.08);
        let pa_w = 280.0; let pa_h = 480.0;
        let pa_x = (w - pa_w) / 2.0;
        let pa_y = (h - pa_h) / 2.0;
        push_panel(&mut verts, &mut indices, pa_x, pa_y, pa_w, pa_h, 0.09);
        let px = pa_x + 12.0;
        let highlight = rgba_to_f32([255, 255, 100, 255]);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 12.0, "PAUSED", highlight, 4.0);
        // Controls
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 56.0, "L-R MOVE", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 80.0, "DN  SOFT DROP", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 104.0, "SPC HARD DROP", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 128.0, "UP  CW  Z CCW", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 152.0, "C   HOLD", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 176.0, "P   RESUME", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 200.0, "N SKIP +- VOL", dim_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 224.0, "F1  THEME", dim_col, 2.0);
        // Settings section
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 264.0, "SETTINGS", highlight, 2.5);
        // Volume
        let vol = if let Ok(audio) = world.audio.try_lock() { audio.volume } else { 0.8 };
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 296.0,
                  &format!("VOL  {:.0}%", vol * 100.0), text_col, 2.0);
        // Theme
        let theme_names = ["DEFAULT", "WATER", "SPACE", "FLOW", "FLUID", "CRYSTAL", "FRACTAL", "DEBUG"];
        let theme_name = theme_names.get(world.theme_index).unwrap_or(&"DEFAULT");
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 322.0,
                  &format!("THEME  {}", theme_name), text_col, 2.0);
        push_text_embossed(&mut verts, &mut indices, px, pa_y + 430.0, "ESC  MENU", dim_col, 2.0);
    }

    // Particles (always visible, not affected by HUD fade)
    world.effects.render_particles(&mut verts, &mut indices);

    // Apply HUD opacity (skip particles — they're always visible)
    let opacity = world.hud_opacity;
    if opacity < 0.99 {
        let particle_verts = world.effects.particles.particles.len() * 4;
        let hud_vert_count = verts.len().saturating_sub(particle_verts);
        for v in verts[..hud_vert_count].iter_mut() {
            v.color[3] *= opacity;
        }
    }

    // Toast (always visible, not affected by HUD fade)
    // Analysis labels (SAMPLING/MAPPED/etc) only on debug theme
    // Theme switch toasts show on all themes
    if world.toast_timer > 0.0 {
        let is_analysis = world.toast_text.starts_with("SAMPLING")
            || world.toast_text.starts_with("MAPPED")
            || world.toast_text.starts_with("RESAMPLING")
            || world.toast_text.starts_with("REMAPPED")
            || world.toast_text.starts_with("ANALYZING");
        let show = if is_analysis { world.theme_index == 7 } else { true }; // 2 = debug
        if show {
            let ta = (world.toast_timer.min(1.0) * 200.0) as u8;
            push_text(&mut verts, &mut indices, w / 2.0 - 60.0, h - 30.0,
                      &world.toast_text, rgba_to_f32([200, 200, 200, ta]), 1.5);
        }
    }

    // Combo counter (always visible, not affected by HUD fade)
    if ef.combo_text && rs.combo_count > 0 {
        let intensity = (rs.combo_count as f32 * 0.15).min(1.0);
        let combo_col = rgba_to_f32([
            255,
            (200.0 - intensity * 100.0) as u8,
            (60.0 - intensity * 60.0) as u8,
            (180.0 + intensity * 75.0) as u8,
        ]);
        let scale = 3.0 + rs.combo_count as f32 * 0.3;
        push_text_embossed(&mut verts, &mut indices,
            w / 2.0 - 50.0, h / 2.0 + 40.0,
            &format!("COMBO {}", rs.combo_count), combo_col, scale.min(6.0));
    }

    // Debug analysis dashboard (debug theme only, always visible)
    if world.theme_index == 7 {
        let band_names = ["SB", "BA", "LM", "MI", "UM", "PR", "BR"];
        let dx = 12.0;
        let dy = 210.0;
        let bar_w = 6.0;   // each sub-bar width
        let pair_w = bar_w * 2.0 + 2.0; // two bars + inner gap
        let pair_gap = 4.0; // gap between band pairs
        let max_h = 55.0;
        let rank_col = rgba_to_f32([255, 220, 80, 255]);
        let live_col = rgba_to_f32([80, 200, 80, 160]); // green for real-time level

        // Energy averages row (blue) + real-time level (green)
        push_text(&mut verts, &mut indices, dx, dy, "ENERGY", rgba_to_f32([120, 160, 200, 200]), 1.2);
        for i in 0..7 {
            let px = dx + i as f32 * (pair_w + pair_gap);
            let by = dy + 16.0;

            // Background for both bars
            push_quad(&mut verts, &mut indices, px, by, pair_w, max_h, rgba_to_f32([20, 20, 40, 150]), 0.01);

            // Left bar: rolling energy average (blue/gold)
            let avg_val = world.analysis.energy_averages[i].min(1.0);
            let avg_h = avg_val * max_h;
            let avg_col = if world.analysis.resolved_ranks.contains(&i) { rank_col } else {
                rgba_to_f32([40, 80, 160, 220])
            };
            push_quad(&mut verts, &mut indices, px, by + max_h - avg_h, bar_w, avg_h, avg_col, 0.02);

            // Right bar: real-time band level (green)
            let live_val = world.analysis.bands[i].min(1.0);
            let live_h = live_val * max_h;
            push_quad(&mut verts, &mut indices, px + bar_w + 2.0, by + max_h - live_h, bar_w, live_h, live_col, 0.02);

            // Band label
            push_text(&mut verts, &mut indices, px, by + max_h + 3.0, band_names[i], rgba_to_f32([150, 150, 180, 180]), 0.9);
        }

        // Confidence row (orange) + real-time beat intensity (green)
        let cy = dy + max_h + 36.0;
        push_text(&mut verts, &mut indices, dx, cy, "CONFIDENCE", rgba_to_f32([200, 160, 120, 200]), 1.2);
        for i in 0..7 {
            let px = dx + i as f32 * (pair_w + pair_gap);
            let by = cy + 16.0;

            push_quad(&mut verts, &mut indices, px, by, pair_w, max_h, rgba_to_f32([20, 20, 40, 150]), 0.01);

            // Left bar: confidence (orange/gold)
            let conf_val = world.analysis.confidence_values[i].min(1.0);
            let conf_h = conf_val * max_h;
            let conf_col = if world.analysis.resolved_ranks[0] == i { rank_col } else {
                rgba_to_f32([160, 80, 40, 220])
            };
            push_quad(&mut verts, &mut indices, px, by + max_h - conf_h, bar_w, conf_h, conf_col, 0.02);

            // Right bar: real-time beat intensity (green)
            let beat_val = world.analysis.band_beat_intensity[i].min(1.0);
            let beat_h = beat_val * max_h;
            push_quad(&mut verts, &mut indices, px + bar_w + 2.0, by + max_h - beat_h, bar_w, beat_h, live_col, 0.02);

            push_text(&mut verts, &mut indices, px, by + max_h + 3.0, band_names[i], rgba_to_f32([150, 150, 180, 180]), 0.9);
        }

        // Resolved ranks display
        let ry = cy + max_h + 36.0;
        let [r1, r2, r3] = world.analysis.resolved_ranks;
        push_text(&mut verts, &mut indices, dx, ry,
            &format!("R1:{} R2:{} R3:{}", band_names[r1], band_names[r2], band_names[r3]),
            rank_col, 1.5);
    }

    (verts, indices)
}
