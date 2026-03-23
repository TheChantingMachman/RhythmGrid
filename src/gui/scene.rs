// Scene building — constructs 3D geometry and 2D HUD from game state.
// Separated from world.rs to keep rendering logic isolated from game logic.

use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::pieces::*;


use super::drawing::*;
use super::theme::*;
use super::world::GameWorld;

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

    // Background geometry (transparent — behind everything)
    build_background(&mut tv, &mut ti, world, gw, gh);

    // Fireworks (transparent, behind board)
    {
        use super::effects::AudioEffect;
        let fx_ctx = super::effects::RenderContext {
            board_width: gw, board_height: gh,
            win_w: THEME.win_w as f32, win_h: THEME.win_h as f32,
            window_aspect: world.window_aspect,
            preview_angle: world.preview_angle,
            hud_opacity: world.hud_opacity,
        };
        if world.effect_flags.fireworks {
            world.fireworks.render(&mut tv, &mut ti, &fx_ctx);
        }
    }

    // Occupied cells as translucent 3D cubes
    let ef = &world.effect_flags;
    for cell in &world.render_board.occupied {
        let band = (cell.type_index as usize) % 7;
        let mut color = rgba_to_f32(world.themed_piece_color(cell.type_index));
        color[3] = 0.75;
        let (band_glow, depth) = if ef.cube_glow {
            (world.bands_norm[band] * 2.0, cube_depth + world.band_beat_intensity[band] * 0.3)
        } else {
            (0.0, cube_depth)
        };
        push_cube_3d(&mut tv, &mut ti, cell.col as f32, cell.row as f32, depth, color, band_glow);
    }

    // Ghost piece
    if ef.ghost_piece {
        for cell in &world.render_board.ghost {
            let base_color = world.themed_piece_color(cell.type_index);
            let ghost_color = rgba_to_f32([base_color[0], base_color[1], base_color[2], 40]);
            push_cube_3d(&mut tv, &mut ti, cell.col as f32, cell.row as f32, cube_depth * 0.2, ghost_color, 0.0);
        }
    }

    // Active piece
    for cell in &world.render_board.active {
        let band = (cell.type_index as usize) % 7;
        let mut color = rgba_to_f32(world.themed_piece_color(cell.type_index));
        color[3] = 0.85;
        let (active_glow, active_depth) = if ef.active_piece_pulse {
            (world.bands_norm[band] * 2.0, cube_depth + world.band_beat_intensity[band] * 0.3)
        } else {
            (0.0, cube_depth)
        };
        push_cube_3d(&mut tv, &mut ti, cell.col as f32, cell.row as f32, active_depth, color, active_glow);
    }

    // Grid lines (effect module)
    if ef.grid_lines {
        use super::effects::AudioEffect;
        let grid_ctx = super::effects::RenderContext {
            board_width: gw, board_height: gh,
            win_w: THEME.win_w as f32, win_h: THEME.win_h as f32,
            window_aspect: world.window_aspect,
            preview_angle: world.preview_angle,
            hud_opacity: world.hud_opacity,
        };
        world.grid_lines.render(&mut sv, &mut si, &grid_ctx);
    }

    // --- 3D Music Dashboard ---
    let hud_a = world.hud_opacity;

    // Volume: [-] ====== [+]  (right side, anchored to window edge)
    let vol = if let Ok(audio) = world.audio.try_lock() { audio.volume } else { 0.5 };
    let audio_x = 12.5;
    let vol_minus_x = audio_x;
    let vol_btn_w = 0.5;
    let vol_bar_x = vol_minus_x + vol_btn_w + 0.15;
    let vol_plus_x = audio_x + 2.5;
    let vol_bar_w = vol_plus_x - vol_bar_x - 0.15;
    let vol_y = 15.5;
    let vol_h = 0.2;
    let vol_bg = rgba_to_f32([15, 15, 30, (160.0 * hud_a) as u8]);
    push_slab_3d(&mut tv, &mut ti, vol_bar_x, vol_y + 0.15, vol_bar_w, vol_h, 0.15, vol_bg);
    let vol_fill = rgba_to_f32([60, 100, 180, (220.0 * hud_a) as u8]);
    push_slab_3d(&mut tv, &mut ti, vol_bar_x, vol_y + 0.15, vol_bar_w * vol, vol_h, 0.3, vol_fill);
    // Vol down button [-]
    let vd_color = if world.btn_hovered(super::world::ButtonId::VolDown) {
        rgba_to_f32([80, 60, 60, (240.0 * hud_a) as u8])
    } else {
        rgba_to_f32([30, 30, 50, (180.0 * hud_a) as u8])
    };
    push_slab_3d(&mut tv, &mut ti, vol_minus_x, vol_y, vol_btn_w, 0.5, 0.4, vd_color);
    // Vol up button [+]
    let vu_color = if world.btn_hovered(super::world::ButtonId::VolUp) {
        rgba_to_f32([60, 80, 60, (240.0 * hud_a) as u8])
    } else {
        rgba_to_f32([30, 30, 50, (180.0 * hud_a) as u8])
    };
    push_slab_3d(&mut tv, &mut ti, audio_x + 2.5, vol_y, vol_btn_w, 0.5, 0.4, vu_color);

    // Transport buttons: [<<] [>||] [>>] [SH]
    // Transport buttons: [<<] [>||] [>>] [SH]
    let transport_ids = [
        super::world::ButtonId::Back,
        super::world::ButtonId::PlayPause,
        super::world::ButtonId::Skip,
        super::world::ButtonId::Shuffle,
    ];
    let is_paused = if let Ok(audio) = world.audio.try_lock() { audio.paused } else { false };
    for &id in &transport_ids {
        let btn = world.buttons.iter().find(|b| b.id == id).unwrap();
        let base_color = match id {
            super::world::ButtonId::PlayPause if is_paused => [50, 80, 50],
            super::world::ButtonId::Shuffle => [50, 50, 80],
            _ => [30, 30, 50],
        };
        let color = if btn.hovered {
            rgba_to_f32([base_color[0] + 40, base_color[1] + 40, base_color[2] + 40, (240.0 * hud_a) as u8])
        } else {
            rgba_to_f32([base_color[0] as u8, base_color[1] as u8, base_color[2] as u8, (180.0 * hud_a) as u8])
        };
        push_slab_3d(&mut tv, &mut ti, btn.world_x, btn.world_y, btn.world_w, btn.world_h, 0.4, color);
    }

    // Folder button (right side, below transport)
    let fld = world.buttons.iter().find(|b| b.id == super::world::ButtonId::Folder).unwrap();
    let fld_color = if fld.hovered {
        rgba_to_f32([60, 80, 140, (240.0 * hud_a) as u8])
    } else {
        rgba_to_f32([30, 40, 70, (180.0 * hud_a) as u8])
    };
    push_slab_3d(&mut tv, &mut ti, fld.world_x, fld.world_y, fld.world_w, fld.world_h, 0.4, fld_color);

    // FFT visualizer (effect module)
    if ef.fft_visualizer {
        use super::effects::AudioEffect;
        let fft_ctx = super::effects::RenderContext {
            board_width: gw, board_height: gh,
            win_w: THEME.win_w as f32, win_h: THEME.win_h as f32,
            window_aspect: world.window_aspect,
            preview_angle: world.preview_angle,
            hud_opacity: hud_a,
        };
        world.fft_vis.render(&mut tv, &mut ti, &fft_ctx);
    }

    // Per-cell clearing animations (shrinking bright cubes)
    if ef.clearing_flash { for cell in &world.clearing_cells {
        if cell.scale > 0.01 {
            // Stay white throughout, fade alpha
            let progress = 1.0 - (cell.timer / super::world::LINE_CLEAR_DURATION).max(0.0);
            let alpha = (1.0 - progress).max(0.0);
            let bright_color = [1.0, 1.0, 1.0, alpha];

            // Render as a scaled cube centered on the cell
            let cx = cell.col as f32 + 0.5;
            let cy = cell.row as f32 + 0.5;
            let half = cell.scale * 0.5;
            let gap = 0.08 * cell.scale;
            let x0 = cx - half + gap;
            let x1 = cx + half - gap;
            let y0 = -(cy - half + gap);
            let y1 = -(cy + half - gap);
            let z0 = 0.0;
            let z1 = cube_depth * cell.scale;
            let n_front = [0.0f32, 0.0, 1.0];

            // Just front face + top face for dissolving cells (simpler, faster)
            let base = tv.len() as u32;
            tv.push(Vertex { position: [x0, y0, z1], normal: n_front, color: bright_color });
            tv.push(Vertex { position: [x1, y0, z1], normal: n_front, color: bright_color });
            tv.push(Vertex { position: [x1, y1, z1], normal: n_front, color: bright_color });
            tv.push(Vertex { position: [x0, y1, z1], normal: n_front, color: bright_color });
            ti.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

            let n_top = [0.0f32, 1.0, 0.0];
            let top_color = [1.0, 1.0, 1.0, alpha * 0.8];
            let base = tv.len() as u32;
            tv.push(Vertex { position: [x0, y0, z1], normal: n_top, color: top_color });
            tv.push(Vertex { position: [x0, y0, z0], normal: n_top, color: top_color });
            tv.push(Vertex { position: [x1, y0, z0], normal: n_top, color: top_color });
            tv.push(Vertex { position: [x1, y0, z1], normal: n_top, color: top_color });
            ti.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }}

    // HUD
    let (hv, hi) = build_hud(world);

    ((sv, si), (tv, ti), (hv, hi))
}

/// Background geometric field: hex grid + connecting web + beat rings
fn build_background(sv: &mut Vec<Vertex>, si: &mut Vec<u32>, world: &GameWorld, gw: f32, gh: f32) {
    let ctx = super::effects::RenderContext {
        board_width: gw,
        board_height: gh,
        win_w: 0.0, win_h: 0.0,
        window_aspect: 1.0,
        preview_angle: world.preview_angle,
        hud_opacity: world.hud_opacity,
    };
    use super::effects::AudioEffect;

    // Hex background (effect module)
    if world.effect_flags.hex_background {
        world.hex_background.render(sv, si, &ctx);
    }

    // Beat rings (effect module)
    if world.effect_flags.beat_rings {
        world.beat_rings.render(sv, si, &ctx);
    }

    // Legacy level-up rings (still inline)
    if !world.effect_flags.level_up_rings { return; }
    let ring_cx = gw / 2.0;
    let ring_cy = -gh / 2.0;
    let ring_z = -1.0;
    let ring_n = [0.0f32, 0.0, 1.0];
    let ring_segments = 32;

    for ring in &world.bg_rings {
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
            sv.push(Vertex { position: [ring_cx + c0 * inner_r, ring_cy + s0 * inner_r, ring_z], normal: ring_n, color: color_inner });
            sv.push(Vertex { position: [ring_cx + c1 * inner_r, ring_cy + s1 * inner_r, ring_z], normal: ring_n, color: color_inner });
            sv.push(Vertex { position: [ring_cx + c1 * outer_r, ring_cy + s1 * outer_r, ring_z], normal: ring_n, color: color_outer });
            sv.push(Vertex { position: [ring_cx + c0 * outer_r, ring_cy + s0 * outer_r, ring_z], normal: ring_n, color: color_outer });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }
}


/// HUD overlay in screen space
fn build_hud(world: &GameWorld) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut indices = Vec::new();
    let ef = &world.effect_flags;
    let t = &THEME;
    let w = t.win_w as f32;
    let h = t.win_h as f32;
    let text_col = rgba_to_f32(t.text_color);
    let dim_col = rgba_to_f32(t.dim_color);

    // --- Fadeable HUD elements (affected by hud_opacity) ---

    let np_x = w - 120.0;
    let np_y = 12.0;

    // Mark where preview piece starts (won't be faded)
    let preview_start_vert = verts.len();
    let next_type_idx = world.session.bag.peek();
    let next_type = TETROMINO_TYPES[next_type_idx];
    let next_cells = piece_cells(next_type, 0);
    let next_color = rgba_to_f32(world.themed_piece_color(next_type_idx as u32));
    // Correct preview scale for window aspect ratio
    let theme_aspect = w / h;
    let aspect_corr = theme_aspect / world.window_aspect;
    let preview_scale = 36.0;
    let cube_half = 0.42;
    let preview_cx = np_x + 54.0;
    let preview_cy = np_y + 52.0;

    // 3-axis rotation
    let ax = world.preview_angle * 0.3;
    let ay = world.preview_angle * 0.7;
    let az = world.preview_angle * 0.15;
    let (sx_r, cx_r) = (ax.sin(), ax.cos());
    let (sy, cy) = (ay.sin(), ay.cos());
    let (sz, cz) = (az.sin(), az.cos());

    let mut center = [0.0f32; 3];
    for &(dr, dc) in &next_cells {
        center[0] += dc as f32;
        center[1] += dr as f32;
    }
    center[0] /= next_cells.len() as f32;
    center[1] /= next_cells.len() as f32;

    for &(dr, dc) in &next_cells {
        let local_x = dc as f32 - center[0];
        let local_y = dr as f32 - center[1];
        let corners_local: [[f32; 3]; 8] = [
            [local_x - cube_half, local_y - cube_half, -cube_half],
            [local_x + cube_half, local_y - cube_half, -cube_half],
            [local_x + cube_half, local_y + cube_half, -cube_half],
            [local_x - cube_half, local_y + cube_half, -cube_half],
            [local_x - cube_half, local_y - cube_half, cube_half],
            [local_x + cube_half, local_y - cube_half, cube_half],
            [local_x + cube_half, local_y + cube_half, cube_half],
            [local_x - cube_half, local_y + cube_half, cube_half],
        ];

        let mut projected = [[0.0f32; 2]; 8];
        for (i, c) in corners_local.iter().enumerate() {
            let y1 = c[1] * cx_r - c[2] * sx_r;
            let z1 = c[1] * sx_r + c[2] * cx_r;
            let x2 = c[0] * cy + z1 * sy;
            let z2 = -c[0] * sy + z1 * cy;
            let x3 = x2 * cz - y1 * sz;
            let y3 = x2 * sz + y1 * cz;
            let persp = 4.0 / (4.0 + z2 * 0.3);
            projected[i] = [
                preview_cx + x3 * preview_scale * persp * aspect_corr,
                preview_cy + y3 * preview_scale * persp,
            ];
        }

        let faces: &[([usize; 4], [f32; 3])] = &[
            ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
            ([5, 1, 2, 6], [1.0, 0.0, 0.0]),
            ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
            ([0, 3, 7, 4], [-1.0, 0.0, 0.0]),
            ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
            ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ];

        let mut face_order: Vec<(usize, f32)> = faces.iter().enumerate().map(|(i, (_, n))| {
            let _ny1 = n[1] * cx_r - n[2] * sx_r;
            let nz1 = n[1] * sx_r + n[2] * cx_r;
            let nz2 = -n[0] * sy + nz1 * cy;
            (i, nz2)
        }).collect();
        face_order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        for &(fi, nz) in &face_order {
            if nz < 0.0 { continue; }
            let (ci, _normal) = &faces[fi];
            let shade = 0.4 + nz * 0.6;
            let fc = [next_color[0] * shade, next_color[1] * shade, next_color[2] * shade, next_color[3]];

            let base = verts.len() as u32;
            for &idx in ci {
                let px = projected[idx];
                let (nx, ny) = px_to_ndc(px[0], px[1], w, h);
                verts.push(Vertex { position: [nx, ny, 0.06], normal: HUD_NORMAL, color: fc });
            }
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    // Held piece preview (top-left, mirrors next piece position)
    if let Some(ref held) = world.render_held {
        let held_cells = held.cells;
        let held_color = rgba_to_f32(world.themed_piece_color(held.type_index));
        let held_cx = 66.0;
        let held_cy = np_y + 52.0;
        let held_scale = 36.0;

        let mut held_center = [0.0f32; 3];
        for &(dr, dc) in &held_cells {
            held_center[0] += dc as f32;
            held_center[1] += dr as f32;
        }
        held_center[0] /= held_cells.len() as f32;
        held_center[1] /= held_cells.len() as f32;

        // Slower rotation than next piece
        let hax = world.preview_angle * 0.2;
        let hay = world.preview_angle * 0.5;
        let (hsx, hcx) = (hax.sin(), hax.cos());
        let (hsy, hcy) = (hay.sin(), hay.cos());

        for &(dr, dc) in &held_cells {
            let lx = dc as f32 - held_center[0];
            let ly = dr as f32 - held_center[1];
            let corners: [[f32; 3]; 8] = [
                [lx - cube_half, ly - cube_half, -cube_half],
                [lx + cube_half, ly - cube_half, -cube_half],
                [lx + cube_half, ly + cube_half, -cube_half],
                [lx - cube_half, ly + cube_half, -cube_half],
                [lx - cube_half, ly - cube_half, cube_half],
                [lx + cube_half, ly - cube_half, cube_half],
                [lx + cube_half, ly + cube_half, cube_half],
                [lx - cube_half, ly + cube_half, cube_half],
            ];
            let mut proj = [[0.0f32; 2]; 8];
            for (i, c) in corners.iter().enumerate() {
                let y1 = c[1] * hcx - c[2] * hsx;
                let z1 = c[1] * hsx + c[2] * hcx;
                let x2 = c[0] * hcy + z1 * hsy;
                let z2 = -c[0] * hsy + z1 * hcy;
                let persp = 4.0 / (4.0 + z2 * 0.3);
                proj[i] = [
                    held_cx + x2 * held_scale * persp * aspect_corr,
                    held_cy + y1 * held_scale * persp,
                ];
            }
            let faces: &[([usize; 4], [f32; 3])] = &[
                ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
                ([5, 1, 2, 6], [1.0, 0.0, 0.0]),
                ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
                ([0, 3, 7, 4], [-1.0, 0.0, 0.0]),
                ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
                ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
            ];
            let mut fo: Vec<(usize, f32)> = faces.iter().enumerate().map(|(i, (_, n))| {
                let nz1 = n[1] * hsx + n[2] * hcx;
                let nz2 = -n[0] * hsy + nz1 * hcy;
                (i, nz2)
            }).collect();
            fo.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            for &(fi, nz) in &fo {
                if nz < 0.0 { continue; }
                let (ci, _) = &faces[fi];
                let shade = 0.4 + nz * 0.6;
                let fc = [held_color[0] * shade, held_color[1] * shade, held_color[2] * shade, held_color[3]];
                let base = verts.len() as u32;
                for &idx in ci {
                    let px = proj[idx];
                    let (nx, ny) = px_to_ndc(px[0], px[1], w, h);
                    verts.push(Vertex { position: [nx, ny, 0.06], normal: HUD_NORMAL, color: fc });
                }
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }

    let preview_end_vert = verts.len();

    // Hold label (top-left, above held piece preview)
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
    if ef.t_spin_flash && world.t_spin_flash > 0.01 {
        let ta = (world.t_spin_flash * 255.0) as u8;
        push_text(&mut verts, &mut indices, w / 2.0 - 40.0, h / 2.0 - 60.0,
                  "T-SPIN", rgba_to_f32([255, 100, 255, ta]), 3.0);
    }

    // Combo counter (only visible during active combo)
    if ef.combo_text && rs.combo_count > 0 {
        let combo_col = rgba_to_f32([255, 200, 60, 255]);
        push_text(&mut verts, &mut indices, 12.0, stats_y + 114.0,
                  &format!("COMBO {}", rs.combo_count), combo_col, 2.0);
    }

    // Toast message (theme switch notification)
    if world.toast_timer > 0.0 {
        let ta = (world.toast_timer.min(1.0) * 255.0) as u8;
        push_text(&mut verts, &mut indices, w / 2.0 - 60.0, h - 30.0,
                  &world.toast_text, rgba_to_f32([200, 200, 200, ta]), 1.5);
    }

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

    // Vol -/+ labels
    let (vd_x, vd_y) = project_label(super::world::ButtonId::VolDown, world);
    push_text(&mut verts, &mut indices, vd_x, vd_y, "-", dim_col, 1.0);
    let (vu_x, vu_y) = project_label(super::world::ButtonId::VolUp, world);
    push_text(&mut verts, &mut indices, vu_x, vu_y, "+", dim_col, 1.0);

    // Transport labels
    let (is_paused_lbl, is_shuffled_lbl) = if let Ok(audio) = world.audio.try_lock() {
        (audio.paused, audio.shuffled)
    } else {
        (false, false)
    };
    let shuffle_label = if is_shuffled_lbl { "SH*" } else { "SH" };
    let transport_labels = [
        (super::world::ButtonId::Back, "<<"),
        (super::world::ButtonId::PlayPause, if is_paused_lbl { ">" } else { "||" }),
        (super::world::ButtonId::Skip, ">>"),
        (super::world::ButtonId::Shuffle, shuffle_label),
    ];
    for (id, label) in transport_labels {
        let col = if id == super::world::ButtonId::Shuffle && is_shuffled_lbl {
            rgba_to_f32([100, 200, 255, 255]) // highlighted when active
        } else if world.btn_hovered(id) {
            text_col
        } else {
            dim_col
        };
        let (lx, ly) = project_label(id, world);
        push_text(&mut verts, &mut indices, lx, ly, label, col, 1.0);
    }

    // Folder label
    let folder_col = if world.btn_hovered(super::world::ButtonId::Folder) { text_col } else { dim_col };
    let (fl_x, fl_y) = project_label(super::world::ButtonId::Folder, world);
    push_text(&mut verts, &mut indices, fl_x - 8.0, fl_y, "FOLDER", folder_col, 1.0);

    // FFT lock label
    let lock_col = if world.fft_locked { text_col } else { dim_col };
    let (ll_x, ll_y) = project_label(super::world::ButtonId::FftLock, world);
    push_text(&mut verts, &mut indices, ll_x - 4.0, ll_y, "LOCK", lock_col, 1.0);

    // State overlays
    if rs.state == GameState::GameOver {
        push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([120, 0, 0, 80]), 0.08);
        let go_w = 200.0; let go_h = 150.0;
        let go_x = (w - go_w) / 2.0;
        let go_y = (h - go_h) / 2.0;
        push_panel(&mut verts, &mut indices, go_x, go_y, go_w, go_h, 0.09);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 8.0, "GAME OVER", rgba_to_f32([255, 80, 80, 255]), 2.0);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 34.0, "SCORE", dim_col, 1.0);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 46.0,
                  &format!("{}", rs.score), text_col, 3.0);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 72.0,
                  &format!("LVL {}  LINES {}", rs.level, rs.total_lines), dim_col, 1.0);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 84.0,
                  &format!("COMBO {}  PCS {}", rs.max_combo, rs.pieces_placed), dim_col, 1.0);
        let mins = (rs.time_played_secs / 60.0) as u32;
        let secs = (rs.time_played_secs % 60.0) as u32;
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 96.0,
                  &format!("TIME {}:{:02}", mins, secs), dim_col, 1.0);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 116.0, "ENTER TO RESTART", dim_col, 1.0);
    }

    if rs.state == GameState::Paused {
        push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([0, 0, 0, 60]), 0.08);
        let pa_w = 130.0; let pa_h = 240.0;
        let pa_x = (w - pa_w) / 2.0;
        let pa_y = (h - pa_h) / 2.0;
        push_panel(&mut verts, &mut indices, pa_x, pa_y, pa_w, pa_h, 0.09);
        let px = pa_x + 8.0;
        let highlight = rgba_to_f32([255, 255, 100, 255]);
        push_text(&mut verts, &mut indices, px, pa_y + 8.0, "PAUSED", highlight, 2.0);
        // Controls
        push_text(&mut verts, &mut indices, px, pa_y + 30.0, "L-R MOVE", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 42.0, "DN  SOFT DROP", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 54.0, "SPC HARD DROP", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 66.0, "UP  CW  Z CCW", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 78.0, "C   HOLD", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 90.0, "P   RESUME", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 102.0, "N SKIP +- VOL", dim_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 114.0, "F1  THEME", dim_col, 1.0);
        // Settings section
        push_text(&mut verts, &mut indices, px, pa_y + 136.0, "SETTINGS", highlight, 1.5);
        // Volume
        let vol = if let Ok(audio) = world.audio.try_lock() { audio.volume } else { 0.8 };
        push_text(&mut verts, &mut indices, px, pa_y + 154.0,
                  &format!("VOL  {:.0}%", vol * 100.0), text_col, 1.0);
        // Theme
        let theme_names = ["DEFAULT", "WATER", "DEBUG"];
        let theme_name = theme_names.get(world.theme_index).unwrap_or(&"DEFAULT");
        push_text(&mut verts, &mut indices, px, pa_y + 166.0,
                  &format!("THEME  {}", theme_name), text_col, 1.0);
        push_text(&mut verts, &mut indices, px, pa_y + 210.0, "ESC  MENU", dim_col, 1.0);
    }

    // Particles (always visible, not affected by HUD fade)
    world.particles.render(&mut verts, &mut indices);

    // Apply HUD opacity — skip preview piece and particles
    let opacity = world.hud_opacity;
    if opacity < 0.99 {
        let particle_verts = world.particles.particles.len() * 4;
        let hud_vert_count = verts.len().saturating_sub(particle_verts);
        for (i, v) in verts[..hud_vert_count].iter_mut().enumerate() {
            // Skip preview piece vertices (always visible)
            if i >= preview_start_vert && i < preview_end_vert {
                continue;
            }
            v.color[3] *= opacity;
        }
    }

    (verts, indices)
}
